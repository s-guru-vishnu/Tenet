// ============================================
// TENET Landing Page — JavaScript
// ============================================

// Navbar scroll effect
const navbar = document.getElementById('navbar');
window.addEventListener('scroll', () => {
  navbar.classList.toggle('scrolled', window.scrollY > 20);
});

// Mobile menu toggle
const mobileMenuBtn = document.getElementById('mobileMenuBtn');
const mobileMenu = document.getElementById('mobileMenu');

mobileMenuBtn.addEventListener('click', () => {
  mobileMenu.classList.toggle('open');
});

// Close mobile menu on link click
mobileMenu.querySelectorAll('a').forEach(link => {
  link.addEventListener('click', () => {
    mobileMenu.classList.remove('open');
  });
});

// Scroll-triggered fade-in animations
const observerOptions = {
  threshold: 0.15,
  rootMargin: '0px 0px -40px 0px'
};

const observer = new IntersectionObserver((entries) => {
  entries.forEach(entry => {
    if (entry.isIntersecting) {
      entry.target.classList.add('visible');
      observer.unobserve(entry.target);
    }
  });
}, observerOptions);

document.querySelectorAll('.feature-card, .step, .tech-item, .download-card').forEach(el => {
  el.classList.add('fade-in');
  observer.observe(el);
});

// Format bytes nicely
function formatBytes(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

// Download icon SVG
const downloadIconSVG = `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>`;

// Fetch and render download buttons
async function loadDownloads() {
  const container = document.getElementById('downloadButtons');
  
  try {
    const res = await fetch('/api/downloads');
    if (!res.ok) throw new Error('API not available');
    const data = await res.json();

    if (!data.downloads || data.downloads.length === 0) {
      throw new Error('No downloads in API');
    }

    renderButtons(data.downloads);

  } catch (err) {
    // FALLBACK: Use local files (for Netlify)
    const localDownloads = [
      {
        name: 'tenet_0.1.0_x64_en-US.msi',
        label: 'Windows Installer (MSI)',
        size: 3657728,
        url: './tenet_0.1.0_x64_en-US.msi'
      },
      {
        name: 'tenet_0.1.0_x64-setup.exe',
        label: 'Windows Setup (EXE)',
        size: 2427997,
        url: './tenet_0.1.0_x64-setup.exe'
      }
    ];
    renderButtons(localDownloads);
  }
}

function renderButtons(downloads) {
  const container = document.getElementById('downloadButtons');
  container.innerHTML = '';

  downloads.forEach((dl, i) => {
    const btn = document.createElement('a');
    btn.href = dl.url;
    btn.className = `download-btn ${i === 0 ? 'primary' : 'secondary'}`;
    btn.innerHTML = `
      ${downloadIconSVG}
      <div>
        <div>${dl.label}</div>
        <div class="btn-meta">${dl.name} — ${formatBytes(dl.size)}</div>
      </div>
    `;
    container.appendChild(btn);
  });
}

// Smooth scroll for anchor links
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
  anchor.addEventListener('click', function(e) {
    e.preventDefault();
    const target = document.querySelector(this.getAttribute('href'));
    if (target) {
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  });
});

// Load downloads on page load
loadDownloads();
