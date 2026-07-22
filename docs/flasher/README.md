# Web flasher (ESP Web Tools)

Published to **GitHub Pages** when a release is published or when the
`Deploy web flasher (GitHub Pages)` workflow is run manually.

## One-time repo setup

**Important:** Pages must use **GitHub Actions**, not “Deploy from a branch”.
Branch deploy serves the README (Jekyll) and the flasher workflow will 404.

1. Open [Settings → Pages](https://github.com/kf106/esp32-c3-supermini-oled-dasher/settings/pages).
2. Under **Build and deployment → Source**, choose **GitHub Actions** (not “Deploy from a branch”).
3. **Settings → Actions → General → Workflow permissions:** **Read and write permissions**.
4. **Actions** → **Deploy web flasher (GitHub Pages)** → **Run workflow**.
5. After a green run, the site is live at
   `https://kf106.github.io/esp32-c3-supermini-oled-dasher/`

You should see **Install firmware**, not the project README.

Public URL: https://kf106.github.io/esp32-c3-supermini-oled-dasher/
