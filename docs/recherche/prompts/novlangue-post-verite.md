# Prompt de recherche — novlangue et post-vérité

Prompt du lot 1 (item #23 du backlog). On le donne tel quel à un agent de
recherche, par exemple une session Claude Code avec la recherche web, depuis la
racine du dépôt. Il complète l'état de l'art
([`../etat-de-l-art.md`](../etat-de-l-art.md)), qui couvre déjà l'esquive en
interview, Bull, Bavelas, Clayman, et la langue de bois au sens de Thom,
Delporte et Krieg-Planque.

---

## Contexte

Bullitik est un jeu vocal en français. Un intervieweur, joué par un LLM local,
pose des questions factuelles à la manière d'une interview politique. Le joueur
répond à voix haute : il doit parler autour du sujet sans répondre, et le plus
discrètement possible. Un juge, lui aussi joué par un LLM, note la réponse : le
joueur a-t-il répondu, est-elle pertinente, est-elle discrète, quelle technique
d'esquive a-t-il employée. La fiche du juge part d'une typologie provisoire de
11 techniques, tirée de l'état de l'art.

Lis d'abord `docs/recherche/etat-de-l-art.md` (typologie, règles de départage,
critères de notation) et `content/juge/fiche-techniques.md`. Ne refais pas ce
travail : cette recherche l'**étend**.

## Objectif

Documenter deux champs que l'état de l'art n'a pas couverts :

1. **La novlangue**, au sens large : la langue qui façonne la pensée en
   restreignant ou en retournant le sens des mots. On l'étudie dans le
   discours politique, administratif et managérial.
2. **La post-vérité**, et en particulier les travaux de **Clément
   Viktorovitch** sur la rhétorique, le discours politique et le rapport des
   discours à la vérité.

En tirer ce qui sert le jeu. D'abord, de nouvelles techniques d'esquive, ou
des variantes de celles de la typologie, avec leurs marqueurs linguistiques.
Ensuite, des critères pour juger la discrétion : ce qu'un auditeur repère, ce
qui passe inaperçu. Enfin, des idées de questions ou de relances pour
l'intervieweur.

## Pistes à explorer

Ce sont des **pistes, pas des faits** : vérifie chaque titre, chaque date et
chaque attribution avant de t'en servir.

- **Novlangue, les sources** : George Orwell, *1984* (1949), et son appendice
  « Les principes du novlangue » ; « Politics and the English Language »
  (1946).
- **Langue des régimes** : Victor Klemperer, *LTI, la langue du
  IIIe Reich* (1947).
- **Novlangue contemporaine en France** : Éric Hazan, *LQR. La propagande du
  quotidien* (2006). Les travaux sur la « novlangue managériale » (par exemple
  Agnès Vandevelde-Rougale). Les « formules » d'Alice Krieg-Planque, déjà
  citée dans l'état de l'art : ne la reprends que pour ce qui est nouveau.
- **Clément Viktorovitch** : ses livres (*Le Pouvoir rhétorique*, et les
  suivants s'il y en a), ses chroniques (franceinfo, Clique, etc.), ses cours
  et conférences, et ce qu'il dit de la post-vérité. Dresse d'abord la liste
  de ses travaux depuis des sources fiables (éditeur, BnF, médias), puis
  sélectionne ce qui touche l'esquive, la manipulation par le langage et le
  rapport à la vérité.
- **Post-vérité, les cadres** : Harry Frankfurt, *On Bullshit* (2005), sur
  l'indifférence à la vérité. Lee McIntyre, *Post-Truth* (2018). Les
  définitions des dictionnaires (Oxford, 2016). Les travaux francophones sur la
  post-vérité.
- Tout autre travail sérieux, en français ou en anglais, que tu trouves en
  chemin : linguistique, rhétorique, analyse du discours, sciences politiques.

## Méthode

- **Sources primaires d'abord.** Le livre, l'article, la chronique elle-même.
  Si tu ne peux citer qu'une source secondaire (critique, Wikipédia, résumé),
  dis-le explicitement, comme l'état de l'art le fait déjà.
- **Chaque affirmation est sourcée**, avec un lien vérifiable : éditeur, DOI,
  catalogue de bibliothèque, page du média. Une source que tu n'as pas pu
  ouvrir est marquée « non consultée ».
- **N'invente rien.** Aucune citation, date, page ni titre qui n'ait été lu
  dans une source. En cas de doute, écris-le.
- **Citations courtes** : une ou deux phrases au plus par source, le reste
  reformulé (droit d'auteur).
- **Neutralité politique.** Le jeu ne vise aucun camp. Prends les exemples dans
  tout le spectre politique et dans plusieurs époques. Signale quand une source
  est elle-même engagée : c'est souvent le cas des essais sur la novlangue.
- **Sépare les constats des propositions.** Ce que disent les sources d'un
  côté, ce que tu proposes pour le jeu de l'autre.

## Livrable

Un fichier `docs/recherche/novlangue-post-verite.md`, en français, dans le
style de `etat-de-l-art.md` : phrases courtes, sections numérotées, sources en
fin de document. Plan attendu :

1. **Novlangue** : origine, mécanismes (réduction du vocabulaire, euphémisme,
   inversion du sens, mots-écrans, nominalisation, sigles…), exemples
   documentés.
2. **Langue des régimes et propagande du quotidien** : Klemperer, Hazan, etc.
   Ne retiens que ce qui ajoute à la section 1.
3. **Clément Viktorovitch** : liste vérifiée de ses travaux, puis ses apports
   sur la rhétorique, l'esquive et la post-vérité.
4. **Post-vérité** : définitions, distinctions (mensonge, bullshit,
   indifférence à la vérité), et ce que cela change pour une interview.
5. **Pour le jeu** :
   - un tableau des procédés repérés, comparé à la typologie actuelle des
     11 techniques : déjà couvert (laquelle ?), variante, ou technique
     nouvelle ;
   - pour chaque procédé nouveau : définition, marqueurs repérables dans une
     transcription, exemple de réponse de joueur, et niveau de discrétion
     (repéré ou non par un auditeur ordinaire, avec la source) ;
   - des idées de relances pour l'intervieweur face à ces procédés ;
   - les limites : ce qu'un juge LLM ne peut pas repérer dans une simple
     transcription (intonation, contexte) ;
   - les questions ouvertes, à trancher avec le responsable du projet.
6. **Sources** : références complètes avec liens, chacune marquée
   « consultée », « source secondaire » ou « non consultée ».

## Ce qu'il ne faut pas faire

- Ne pas modifier la typologie ni la fiche du juge (`content/`) : ce sera
  l'item #11, après validation de la méthode de recherche (#22).
- Ne pas recopier l'état de l'art : n'y renvoyer que pour situer un apport
  nouveau.
- Ne pas prendre parti sur les personnalités ou partis cités en exemple.
