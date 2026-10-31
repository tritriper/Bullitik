//! Audio des spikes : enregistrer au micro, lire sur les haut-parleurs,
//! lire et écrire des WAV, convertir en mono et rééchantillonner.
//!
//! Tout le son circule en `Clip` : échantillons mono `f32` dans [-1, 1] et
//! leur fréquence d'échantillonnage.

use anyhow::{Context, Result, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use std::io::BufRead;
use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Un extrait sonore mono.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    pub samples: Vec<f32>,
    pub rate: u32,
}

impl Clip {
    pub fn duration(&self) -> Duration {
        if self.rate == 0 {
            return Duration::ZERO;
        }
        Duration::from_secs_f64(self.samples.len() as f64 / f64::from(self.rate))
    }

    /// Le même extrait à une autre fréquence d'échantillonnage.
    pub fn resampled(&self, rate: u32) -> Clip {
        Clip {
            samples: resample(&self.samples, self.rate, rate),
            rate,
        }
    }
}

/// Moyenne des canaux d'un flux entrelacé.
pub fn to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Rééchantillonnage par sinc fenêtré (Hann), avec filtre anti-repliement
/// quand on descend en fréquence. Pensé pour des extraits de quelques
/// secondes à quelques minutes, pas pour le temps réel.
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    if from == 0 || to == 0 {
        return Vec::new();
    }
    let ratio = f64::from(to) / f64::from(from);
    // Fréquence de coupure, relative à la fréquence de Nyquist de l'entrée.
    let cutoff = ratio.min(1.0) * 0.95;
    // Demi-largeur du noyau, en échantillons d'entrée.
    let half = (16.0 / cutoff).ceil();
    let out_len = (input.len() as f64 * ratio).round() as usize;
    let last = input.len() as isize - 1;
    (0..out_len)
        .map(|n| {
            let t = n as f64 / ratio;
            let first = (t - half).ceil().max(0.0) as isize;
            let end = ((t + half).floor() as isize).min(last);
            let mut acc = 0.0;
            let mut norm = 0.0;
            for k in first..=end {
                let x = t - k as f64;
                let w = 0.5 + 0.5 * (std::f64::consts::PI * x / half).cos();
                let s = if x == 0.0 {
                    1.0
                } else {
                    let a = std::f64::consts::PI * cutoff * x;
                    a.sin() / a
                };
                let c = w * s;
                acc += c * f64::from(input[k as usize]);
                norm += c;
            }
            if norm == 0.0 {
                0.0
            } else {
                (acc / norm) as f32
            }
        })
        .collect()
}

/// Lit un WAV (entier 16/24/32 bits ou flottant), converti en mono.
pub fn read_wav(path: &Path) -> Result<Clip> {
    let mut reader =
        hound::WavReader::open(path).with_context(|| format!("lecture de {}", path.display()))?;
    let spec = reader.spec();
    anyhow::ensure!(
        spec.sample_rate > 0,
        "{} : fréquence d'échantillonnage nulle",
        path.display()
    );
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|s| s as f32 * scale))
                .collect::<Result<_, _>>()?
        }
    };
    Ok(Clip {
        samples: to_mono(&interleaved, usize::from(spec.channels)),
        rate: spec.sample_rate,
    })
}

/// Écrit un WAV mono 16 bits.
pub fn write_wav(path: &Path, clip: &Clip) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: clip.rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("écriture de {}", path.display()))?;
    for &s in &clip.samples {
        writer.write_sample((s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)?;
    }
    writer.finalize()?;
    Ok(())
}

/// Nom du micro par défaut, pour les rapports.
pub fn input_device_name() -> Result<String> {
    let device = cpal::default_host()
        .default_input_device()
        .context("aucun micro trouvé")?;
    Ok(device
        .id()
        .map(|id| id.to_string())
        .unwrap_or_else(|_| "inconnu".into()))
}

/// Enregistre le micro par défaut jusqu'à ce que l'utilisateur appuie sur
/// Entrée. Renvoie l'extrait mono à la fréquence du micro.
pub fn record_until_enter() -> Result<Clip> {
    let device = cpal::default_host()
        .default_input_device()
        .context("aucun micro trouvé")?;
    let supported = device
        .default_input_config()
        .context("configuration du micro illisible")?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let channels = usize::from(config.channels);
    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));

    let stream = match format {
        SampleFormat::F32 => input_stream::<f32>(&device, &config, &buffer)?,
        SampleFormat::I16 => input_stream::<i16>(&device, &config, &buffer)?,
        SampleFormat::I32 => input_stream::<i32>(&device, &config, &buffer)?,
        SampleFormat::U16 => input_stream::<u16>(&device, &config, &buffer)?,
        other => bail!("format de micro non pris en charge : {other}"),
    };
    stream.play()?;
    std::io::stdin().lock().read_line(&mut String::new())?;
    drop(stream);

    let interleaved = std::mem::take(&mut *buffer.lock().expect("verrou du micro"));
    Ok(Clip {
        samples: to_mono(&interleaved, channels),
        rate: config.sample_rate,
    })
}

fn input_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: &Arc<Mutex<Vec<f32>>>,
) -> Result<cpal::Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let buffer = Arc::clone(buffer);
    Ok(device.build_input_stream(
        *config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            if let Ok(mut b) = buffer.lock() {
                b.extend(data.iter().map(|&s| f32::from_sample(s)));
            }
        },
        |err| eprintln!("micro : {err}"),
        None,
    )?)
}

/// Joue un extrait sur la sortie par défaut et attend la fin de la lecture.
pub fn play(clip: &Clip) -> Result<()> {
    let device = cpal::default_host()
        .default_output_device()
        .context("aucune sortie audio trouvée")?;
    let supported = device
        .default_output_config()
        .context("configuration de la sortie audio illisible")?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let mut samples = clip.resampled(config.sample_rate).samples;
    // Silence de garde : « fini » veut dire « dernier échantillon envoyé », pas
    // « joué » ; sans cette marge, la fin peut rester dans le tampon de la carte.
    samples.extend(std::iter::repeat_n(
        0.0,
        config.sample_rate as usize * 3 / 10,
    ));
    let (done_tx, done_rx) = mpsc::channel();

    let stream = match format {
        SampleFormat::F32 => output_stream::<f32>(&device, &config, samples, done_tx)?,
        SampleFormat::I16 => output_stream::<i16>(&device, &config, samples, done_tx)?,
        SampleFormat::I32 => output_stream::<i32>(&device, &config, samples, done_tx)?,
        SampleFormat::U16 => output_stream::<u16>(&device, &config, samples, done_tx)?,
        other => bail!("format de sortie non pris en charge : {other}"),
    };
    stream.play()?;
    // Fin des échantillons, plus une marge pour vider le tampon de la carte son.
    let timeout = clip.duration() + Duration::from_secs(2);
    let _ = done_rx.recv_timeout(timeout);
    std::thread::sleep(Duration::from_millis(200));
    Ok(())
}

fn output_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    samples: Vec<f32>,
    done: mpsc::Sender<()>,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let mut next = 0usize;
    Ok(device.build_output_stream(
        *config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let value = samples.get(next).copied().unwrap_or(0.0);
                next += 1;
                frame.fill(T::from_sample(value));
            }
            if next >= samples.len() {
                let _ = done.send(());
            }
        },
        |err| eprintln!("sortie audio : {err}"),
        None,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: u32, secs: f32) -> Vec<f32> {
        (0..(rate as f32 * secs) as usize)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin() * 0.5)
            .collect()
    }

    /// Fréquence dominante estimée par passages à zéro.
    fn zero_crossing_freq(samples: &[f32], rate: u32) -> f32 {
        let crossings = samples
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count();
        crossings as f32 * rate as f32 / samples.len() as f32
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    #[test]
    fn mono_moyenne_les_canaux() {
        assert_eq!(to_mono(&[1.0, 0.0, 0.5, 0.5], 2), vec![0.5, 0.5]);
    }

    #[test]
    fn descente_48k_vers_16k_garde_la_frequence_et_le_niveau() {
        let out = resample(&sine(440.0, 48_000, 1.0), 48_000, 16_000);
        assert_eq!(out.len(), 16_000);
        let core = &out[1000..15_000];
        assert!((zero_crossing_freq(core, 16_000) - 440.0).abs() < 5.0);
        assert!((rms(core) - 0.5 / 2f32.sqrt()).abs() < 0.01);
    }

    #[test]
    fn montee_22050_vers_48k_garde_la_frequence() {
        let out = resample(&sine(300.0, 22_050, 1.0), 22_050, 48_000);
        assert_eq!(out.len(), 48_000);
        assert!((zero_crossing_freq(&out[2000..46_000], 48_000) - 300.0).abs() < 5.0);
    }

    #[test]
    fn descente_coupe_ce_qui_depasse_nyquist() {
        // 10 kHz n'existe pas à 16 kHz (Nyquist 8 kHz) : il doit disparaître.
        let out = resample(&sine(10_000.0, 48_000, 0.5), 48_000, 16_000);
        assert!(
            rms(&out[500..7500]) < 0.02,
            "repliement : {}",
            rms(&out[500..7500])
        );
    }

    #[test]
    fn frequence_nulle_sans_panique() {
        assert!(resample(&[0.1, 0.2], 0, 16_000).is_empty());
        assert!(resample(&[0.1, 0.2], 16_000, 0).is_empty());
        assert_eq!(
            Clip {
                samples: vec![0.0; 10],
                rate: 0
            }
            .duration(),
            Duration::ZERO
        );
    }

    #[test]
    fn wav_aller_retour() {
        let dir = std::env::temp_dir().join(format!("bullitik-audio-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("x.wav");
        let clip = Clip {
            samples: vec![0.0, 0.5, -0.5, 0.25],
            rate: 22_050,
        };
        write_wav(&path, &clip).unwrap();
        let back = read_wav(&path).unwrap();
        assert_eq!(back.rate, 22_050);
        for (a, b) in clip.samples.iter().zip(&back.samples) {
            assert!((a - b).abs() < 1e-3);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
