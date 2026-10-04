/**
 * 高 DPI 显示优化工具
 * 用于提升图表、Canvas 和 UI 元素在高分辨率显示器上的清晰度
 */

/**
 * 获取当前设备的像素比率
 * 用于高清渲染
 */
export function getDevicePixelRatio(): number {
  return window.devicePixelRatio || 1;
}

/**
 * 为 Canvas 设置高 DPI 支持
 * 使 Canvas 在高分辨率显示器上渲染更清晰
 */
export function setupHighDpiCanvas(canvas: HTMLCanvasElement): void {
  const dpr = getDevicePixelRatio();
  const rect = canvas.getBoundingClientRect();

  // 设置 Canvas 的实际像素大小
  canvas.width = rect.width * dpr;
  canvas.height = rect.height * dpr;

  // 设置 Canvas 的显示大小
  canvas.style.width = `${rect.width}px`;
  canvas.style.height = `${rect.height}px`;

  // 缩放绘图上下文
  const ctx = canvas.getContext('2d');
  if (ctx) {
    ctx.scale(dpr, dpr);
  }
}

/**
 * 获取适合当前 DPI 的字体大小
 */
export function getScaledFontSize(baseSizePx: number): number {
  const dpr = getDevicePixelRatio();
  // 在高 DPI 下适当增大字体，但不要过大
  const scale = Math.min(dpr, 2);
  return Math.round(baseSizePx * scale);
}

/**
 * Recharts 图表的高 DPI 配置
 */
export const highDpiChartConfig = {
  // 增加图表渲染质量
  width: '100%',
  height: '100%',
  // 文本渲染优化
  style: {
    fontSmooth: 'always',
    WebkitFontSmoothing: 'antialiased',
    MozOsxFontSmoothing: 'grayscale',
  },
};

/**
 * 为图表标签提供更清晰的字体配置
 */
export function getChartLabelStyle(baseFontSize = 13): React.CSSProperties {
  return {
    fontSize: `${baseFontSize}px`,
    fontWeight: 500,
    WebkitFontSmoothing: 'antialiased',
    MozOsxFontSmoothing: 'grayscale',
  };
}
