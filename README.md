# SysmonWidget

Un overlay de monitoring système **ultra-léger et moderne** pour Windows 10 / 11.  
Conçu avec Tauri, Rust et Svelte 5, il affiche vos métriques système dans un widget frameless avec un effet **glassmorphism** qui se fond parfaitement sur votre bureau.

---

## ✨ Points Forts

| | |
|---|---|
| ⚡ **Performance** | ~10 Mo de RAM grâce au backend Rust |
| 🎨 **Personnalisation** | Couleurs, transparence, visibilité par ligne, intervalle de polling |
| 🖥️ **UI Moderne** | Interface glassmorphism avec animations fluides (Svelte 5) |
| 🛠️ **Tout-en-un** | CPU, GPU, RAM, Disques, Réseau dans un seul widget discret |
| 📦 **Auto-suffisant** | Mise à jour automatique intégrée |
| 👻 **Discret** | Always-on-bottom — ne gêne jamais vos fenêtres ni vos jeux |

---

## 📊 Métriques Monitorées

| Composant | Détails Affichés |
|---|---|
| **CPU** | % Utilisation · Température (°C) · Sparkline · Top Process 🔥 |
| **GPU** | % Utilisation · Température (°C) · VRAM (Utilisée / Totale) |
| **RAM** | % Utilisation · Go (Utilisés / Total) |
| **Disques** | % Utilisation par partition (C:, D:, E:…) · Go (Utilisés / Total) |
| **Réseau** | Débit instantané Upload ↑ et Download ↓ |

---

## 📥 Installation Rapide

1. Rendez-vous sur la page [**Releases**](https://github.com/CorentinG21/sys-widget/releases/latest)
2. Téléchargez l'installateur **`SysmonWidget_x.x.x_x64-setup.exe`**
3. Lancez l'installation et suivez les étapes
4. Le widget démarre automatiquement après l'installation

> **Note :** L'application demande les droits administrateur — requis pour accéder aux sondes thermiques (CPU/GPU) via LibreHardwareMonitor.

### Prérequis

- **Windows 10 ou 11** (64-bit)
- **WebView2 Runtime** — déjà intégré à Windows 11. Sur Windows 10, [téléchargez-le ici](https://developer.microsoft.com/fr-fr/microsoft-edge/webview2/) si le widget ne s'affiche pas (souvent déjà présent via Edge / mises à jour Windows)

---

## 🛠 Utilisation

### Déplacer le widget

**Clic gauche + glisser** sur le widget pour le repositionner. La position est sauvegardée automatiquement.

### Menu Contextuel

**Clic droit** sur le widget ou sur l'icône dans la zone de notification (systray) :

| Option | Description |
|---|---|
| ⚙ Paramètres… | Ouvre le panneau de configuration |
| Rechercher une mise à jour | Vérifie manuellement si une version plus récente est disponible |
| Démarrer avec Windows | Active/désactive le lancement automatique au démarrage |
| Redémarrer | Relance le widget |
| Quitter | Ferme l'application |

### Panneau Paramètres

Accessible via **Clic droit → Paramètres** :

- **Couleur** — Presets (Cyan Néon, Vert Matrix, Blanc Épuré) ou roue chromatique pour une couleur sur mesure
- **Transparence** — Slider continu de 20 % à 98 %
- **Afficher les détails** — Affiche/masque les sous-lignes (températures, Go RAM…)
- **Verrouiller la position** — Empêche les déplacements accidentels
- **Toujours au premier plan** — Le widget passe au-dessus de toutes les fenêtres (désactivé par défaut)
- **Lignes visibles** — Activez/désactivez chaque métrique individuellement
- **Polling** — Fréquence de rafraîchissement : 1s, 2s ou 5s
- **Ancrage rapide** — Colle le widget dans un coin de l'écran en un clic

---

## 🔄 Mises à Jour

Le widget vérifie automatiquement les mises à jour **30 secondes après le démarrage**, puis **toutes les heures**. Si une mise à jour est disponible, une bannière apparaît directement dans le widget — cliquez dessus pour installer.

Vous pouvez aussi déclencher une vérification via **Clic droit → Rechercher une mise à jour**.

---

## ❓ FAQ

| Problème | Solution |
|---|---|
| **Températures manquantes** | Le widget doit tourner en tant qu'administrateur. Relancez via le menu contextuel → Redémarrer |
| **GPU affiche N/A** | Assurez-vous d'avoir les derniers pilotes graphiques installés |
| **Le widget "disparaît"** | Comportement normal — il reste sous toutes les fenêtres. Activez **"Toujours au premier plan"** dans les Paramètres pour l'inverse |
| **Alerte antivirus** | Faux positif courant dû aux privilèges admin. Le code source est entièrement disponible sur ce dépôt |
| **Le widget reste au-dessus des jeux ?** | Par défaut non. Activez **"Toujours au premier plan"** dans les Paramètres si vous voulez le voir en jeu ou sur une autre fenêtre |

---

## 🗑 Désinstallation

**Paramètres Windows → Applications → SysmonWidget → Désinstaller**

---

## 💻 Stack Technique

| Couche | Technologie |
|---|---|
| **Backend** | Rust · sysinfo · tokio |
| **Frontend** | Svelte 5 · TypeScript |
| **Framework** | Tauri v2 |
| **Capteurs thermiques** | LibreHardwareMonitor (subprocess PowerShell sécurisé) |
| **Build / Release** | GitHub Actions · tauri-action |

---

## 🤝 Contribuer

Les contributions sont les bienvenues !

1. Fork le projet
2. Crée ta branche (`git checkout -b feature/MaFeature`)
3. Commit tes changements (`git commit -m 'feat: MaFeature'`)
4. Push sur la branche (`git push origin feature/MaFeature`)
5. Ouvre une Pull Request

---

## 📝 Licence

Distribué sous licence **MIT**.

---

## 🚀 Liens

[Releases](https://github.com/CorentinG21/sys-widget/releases) · [Tauri v2](https://v2.tauri.app/) · [Issues](https://github.com/CorentinG21/sys-widget/issues)

> 💡 **Astuce** : Pour un look ultra-intégré, essayez la transparence à 30 % avec la couleur "Blanc Épuré" dans les paramètres !
