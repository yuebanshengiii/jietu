// Screen Translator - 前端逻辑
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { clipboard } from '@tauri-apps/plugin-clipboard-manager';

/** 状态 */
let selection = { x: 0, y: 0, w: 0, h: 0, active: false };
let dragStart = null;
let lastTooltipPos = { x: 0, y: 0 };

const $ = (id) => document.getElementById(id);
const sel = $('selector');
const canvas = $('selection-canvas');
const ctx = canvas.getContext('2d');
const box = $('selection-box');
const toolbar = $('selection-toolbar');
const hint = $('selection-hint');

/** 初始化 Canvas 尺寸 */
function resizeCanvas() {
  canvas.width = window.innerWidth * devicePixelRatio;
  canvas.height = window.innerHeight * devicePixelRatio;
  canvas.style.width = window.innerWidth + 'px';
  canvas.style.height = window.innerHeight + 'px';
}
window.addEventListener('resize', resizeCanvas);
resizeCanvas();

/** 绘制遮罩 + 选区高亮 */
function drawOverlay() {
  const { x, y, w, h, active } = selection;
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  
  // 全屏半透明遮罩
  ctx.fillStyle = 'rgba(0, 0, 0, 0.3)';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  
  if (active && w > 0 && h > 0) {
    const dpr = devicePixelRatio;
    // 清除选区内的遮罩
    ctx.clearRect(x * dpr, y * dpr, w * dpr, h * dpr);
    // 选区边框
    ctx.strokeStyle = '#89b4fa';
    ctx.lineWidth = 2 * dpr;
    ctx.strokeRect(x * dpr + 1, y * dpr + 1, w * dpr - 2, h * dpr - 2);
    // 角标
    const corner = 20 * dpr;
    ctx.lineWidth = 3 * dpr;
    ctx.beginPath();
    // 左上
    ctx.moveTo(x * dpr, (y + corner) * dpr);
    ctx.lineTo(x * dpr, y * dpr);
    ctx.lineTo((x + corner) * dpr, y * dpr);
    // 右上
    ctx.moveTo((x + w - corner) * dpr, y * dpr);
    ctx.lineTo((x + w) * dpr, y * dpr);
    ctx.lineTo((x + w) * dpr, (y + corner) * dpr);
    // 右下
    ctx.moveTo((x + w) * dpr, (y + h - corner) * dpr);
    ctx.lineTo((x + w) * dpr, (y + h) * dpr);
    ctx.lineTo((x + w - corner) * dpr, (y + h) * dpr);
    // 左下
    ctx.moveTo((x + corner) * dpr, (y + h) * dpr);
    ctx.lineTo(x * dpr, (y + h) * dpr);
    ctx.lineTo(x * dpr, (y + h - corner) * dpr);
    ctx.stroke();
  }
}

/** 显示选择器 */
function showSelector() {
  sel.classList.remove('hidden');
  selection = { x: 0, y: 0, w: 0, h: 0, active: false };
  box.classList.add('hidden');
  toolbar.classList.add('hidden');
  drawOverlay();
}

/** 隐藏选择器 */
function hideSelector() {
  sel.classList.add('hidden');
  selection.active = false;
  drawOverlay();
}

/** 更新选区几何 */
function updateSelection(x, y, w, h) {
  selection.x = Math.max(0, Math.min(x, window.innerWidth));
  selection.y = Math.max(0, Math.min(y, window.innerHeight));
  selection.w = Math.max(0, Math.min(w, window.innerWidth - selection.x));
  selection.h = Math.max(0, Math.min(h, window.innerHeight - selection.y));
  
  if (selection.w > 10 && selection.h > 10) {
    selection.active = true;
    const dpr = devicePixelRatio;
    box.style.left = selection.x + 'px';
    box.style.top = selection.y + 'px';
    box.style.width = selection.w + 'px';
    box.style.height = selection.h + 'px';
    box.classList.remove('hidden');
    
    // 工具栏位置（选区右上角）
    toolbar.style.left = (selection.x + selection.w) + 'px';
    toolbar.style.top = Math.max(10, selection.y - 44) + 'px';
    toolbar.classList.remove('hidden');
  } else {
    selection.active = false;
    box.classList.add('hidden');
    toolbar.classList.add('hidden');
  }
  drawOverlay();
}

/** 鼠标事件 */
canvas.addEventListener('mousedown', (e) => {
  if (!selection.active) {
    dragStart = { x: e.clientX, y: e.clientY };
    updateSelection(e.clientX, e.clientY, 0, 0);
  }
});

window.addEventListener('mousemove', (e) => {
  if (dragStart) {
    const x = Math.min(dragStart.x, e.clientX);
    const y = Math.min(dragStart.y, e.clientY);
    const w = Math.abs(e.clientX - dragStart.x);
    const h = Math.abs(e.client.y - dragStart.y);
    updateSelection(x, y, w, h);
  }
});

window.addEventListener('mouseup', () => {
  dragStart = null;
});

/** 键盘事件 */
window.addEventListener('keydown', (e) => {
  if (sel.classList.contains('hidden')) return;
  
  if (e.key === 'Escape') {
    hideSelector();
  } else if (e.key === 'Enter' && selection.active) {
    doOCR();
  }
});

/** 工具栏按钮 */
$('btn-ocr').addEventListener('click', doOCR);
$('btn-cancel').addEventListener('click', hideSelector);

/** 执行 OCR + 翻译 */
async function doOCR() {
  if (!selection.active) return;
  
  hideSelector();
  
  try {
    // 截图选区
    const imageData = await captureSelection();
    if (!imageData) throw new Error('截图失败');
    
    // OCR
    const { text } = await invoke('recognize_image', { 
      request: { image_base64: imageData } 
    });
    
    if (!text || !text.trim()) {
      showToast('未识别到文字');
      return;
    }
    
    // 翻译
    const { translated } = await invoke('translate_text', {
      request: { text, source_lang: 'en', target_lang: 'zh' }
    });
    
    // 显示 tooltip
    showTooltip(text, translated);
    
  } catch (err) {
    console.error(err);
    showToast('识别/翻译失败: ' + err.message);
  }
}

/** 截取选区 */
function captureSelection() {
  return new Promise((resolve) => {
    // 创建临时 canvas 绘制选区
    const tempCanvas = document.createElement('canvas');
    const tempCtx = tempCanvas.getContext('2d');
    const dpr = devicePixelRatio;
    
    tempCanvas.width = selection.w * dpr;
    tempCanvas.height = selection.h * dpr;
    
    // 从屏幕截图 (需要 Tauri 权限)
    // 这里用 html2canvas 方案，实际应用中建议用 Tauri 截图 API
    // 简化版：通过插件截图
    invoke('capture_area', {
      x: Math.round(selection.x),
      y: Math.round(selection.y),
      width: Math.round(selection.w),
      height: Math.round(selection.h)
    }).then(resolve).catch(() => resolve(null));
  });
}

/** 显示翻译 tooltip */
function showTooltip(original, translated) {
  const tip = $('tooltip');
  $('tooltip-original').textContent = original;
  $('tooltip-translated').textContent = translated;
  
  // 位置：鼠标附近，防止越界
  const x = Math.min(lastTooltipPos.x, window.innerWidth - 340);
  const y = Math.min(lastTooltipPos.y, window.innerHeight - 220);
  
  invoke('show_tooltip', { x, y, original, translated });
  tip.classList.remove('hidden');
}

/** 隐藏 tooltip */
function hideTooltip() {
  $('tooltip').classList.add('hidden');
  invoke('hide_tooltip');
}

/** 复制功能 */
document.querySelectorAll('.copy-btn').forEach(btn => {
  btn.addEventListener('click', async (e) => {
    const target = e.currentTarget.dataset.target;
    const text = target === 'original' 
      ? $('tooltip-original').textContent 
      : $('tooltip-translated').textContent;
    await clipboard.writeText(text);
    showToast('已复制');
  });
});

$('tooltip-copy-both').addEventListener('click', async () => {
  const original = $('tooltip-original').textContent;
  const translated = $('tooltip-translated').textContent;
  await clipboard.writeText(`${original}\n\n---\n\n${translated}`);
  showToast('已复制全部');
});

$('tooltip-retranslate').addEventListener('click', async () => {
  const original = $('tooltip-original').textContent;
  if (!original) return;
  try {
    const { translated } = await invoke('translate_text', {
      request: { text: original, source_lang: 'en', target_lang: 'zh' }
    });
    $('tooltip-translated').textContent = translated;
    showToast('重新翻译完成');
  } catch (err) {
    showToast('重新翻译失败');
  }
});

$('tooltip-close').addEventListener('click', hideTooltip);

/** 简单 Toast */
function showToast(msg) {
  const toast = document.createElement('div');
  toast.textContent = msg;
  toast.style.cssText = `
    position: fixed; bottom: 30px; left: 50%; transform: translateX(-50%);
    background: #181825; color: #cdd6f4; padding: 10px 20px;
    border: 1px solid #313244; border-radius: 8px; font-size: 13px;
    box-shadow: 0 4px 20px rgba(0,0,0,0.5); z-index: 2147483648;
    animation: slideIn 0.2s ease;
  `;
  document.body.appendChild(toast);
  setTimeout(() => toast.remove(), 2000);
}

/** 监听 Tauri 事件 */
listen('start-selection', () => showSelector()).then(unlisten => {
  window._unlistenStart = unlisten;
});

listen('show-content', (e) => {
  const { original, translated } = e.payload;
  showTooltip(original, translated);
}).then(unlisten => {
  window._unlistenShow = unlisten;
});

/** 记录鼠标位置用于 tooltip 定位 */
window.addEventListener('mousemove', (e) => {
  lastTooltipPos = { x: e.clientX + 16, y: e.clientY + 16 };
});

console.log('Screen Translator ready');