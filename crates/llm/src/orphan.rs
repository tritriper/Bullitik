//! llama-server ne doit pas survivre au spike : orphelin, il garderait
//! plusieurs Go de VRAM et fausserait le lancement suivant. `Drop` suffit pour
//! une sortie normale ; ce module couvre l'arrêt brutal (Gestionnaire des
//! tâches, SIGKILL, bouton « Stop » d'un débogueur).
//!
//!   - Linux : PR_SET_PDEATHSIG, le noyau tue le fils quand le parent meurt.
//!   - Windows : Job Object KILL_ON_JOB_CLOSE ; le handle du job se ferme avec
//!     le processus du spike, et Windows tue alors llama-server.

use std::process::{Child, Command};

/// À appeler sur la commande avant `spawn`.
pub fn configure(cmd: &mut Command) {
    #[cfg(target_os = "linux")]
    linux::configure(cmd);
    #[cfg(not(target_os = "linux"))]
    let _ = cmd;
}

/// À appeler sur le processus juste après `spawn`.
pub fn attach(child: &Child) -> std::io::Result<()> {
    #[cfg(windows)]
    return windows::attach(child);
    #[cfg(not(windows))]
    {
        let _ = child;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
mod linux {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    pub fn configure(cmd: &mut Command) {
        // SAFETY : entre fork et exec, on n'appelle que prctl, qui est
        // async-signal-safe ; aucun verrou ni allocation.
        unsafe {
            cmd.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows {
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };

    pub fn attach(child: &Child) -> std::io::Result<()> {
        // SAFETY : appels Win32 documentés, sur un handle de job que l'on vient
        // de créer et le handle du processus fils, valide tant que `child` vit.
        // Le handle du job n'est jamais fermé volontairement : c'est sa fermeture
        // à la fin du spike qui tue llama-server.
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&raw const info).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                return Err(std::io::Error::last_os_error());
            }
            if AssignProcessToJobObject(job, child.as_raw_handle()) == 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
        Ok(())
    }
}
