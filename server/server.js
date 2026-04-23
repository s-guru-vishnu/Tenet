const express = require('express');
const cors = require('cors');
const path = require('path');
const fs = require('fs');

const app = express();
const PORT = process.env.PORT || 3000;

app.use(cors());
app.use(express.json());

// Serve static client files
app.use(express.static(path.join(__dirname, '..', 'client')));

// API: Get available downloads
app.get('/api/downloads', (req, res) => {
  const releasePath = path.join(__dirname, '..', 'src-tauri', 'target', 'release', 'bundle');
  const downloads = [];

  // Check for MSI installer
  const msiDir = path.join(releasePath, 'msi');
  if (fs.existsSync(msiDir)) {
    const msiFiles = fs.readdirSync(msiDir).filter(f => f.endsWith('.msi'));
    msiFiles.forEach(file => {
      const stats = fs.statSync(path.join(msiDir, file));
      downloads.push({
        name: file,
        type: 'msi',
        label: 'Windows Installer (MSI)',
        size: stats.size,
        url: `/download/msi/${file}`
      });
    });
  }

  // Check for NSIS installer
  const nsisDir = path.join(releasePath, 'nsis');
  if (fs.existsSync(nsisDir)) {
    const nsisFiles = fs.readdirSync(nsisDir).filter(f => f.endsWith('.exe'));
    nsisFiles.forEach(file => {
      const stats = fs.statSync(path.join(nsisDir, file));
      downloads.push({
        name: file,
        type: 'nsis',
        label: 'Windows Setup (EXE)',
        size: stats.size,
        url: `/download/nsis/${file}`
      });
    });
  }

  res.json({
    version: '0.1.0',
    downloads
  });
});

// Download endpoint
app.get('/download/:type/:filename', (req, res) => {
  const { type, filename } = req.params;
  const releasePath = path.join(__dirname, '..', 'src-tauri', 'target', 'release', 'bundle');
  
  let filePath;
  if (type === 'msi') {
    filePath = path.join(releasePath, 'msi', filename);
  } else if (type === 'nsis') {
    filePath = path.join(releasePath, 'nsis', filename);
  } else {
    return res.status(400).json({ error: 'Invalid download type' });
  }

  if (!fs.existsSync(filePath)) {
    return res.status(404).json({ error: 'File not found' });
  }

  res.download(filePath);
});

// Fallback to client index.html
app.get('/{*splat}', (req, res) => {
  res.sendFile(path.join(__dirname, '..', 'client', 'index.html'));
});

app.listen(PORT, () => {
  console.log(`\n  ⏳ TENET Download Server`);
  console.log(`  ➜  Local:  http://localhost:${PORT}`);
  console.log(`  ➜  Ready to serve downloads\n`);
});
