import { useEffect, useRef } from "react";

/* 对话监控活动条：Claude 思考态那样的小正方体网格闪光。
   6 行小方块，右侧完全点亮、向左逐渐灰暗（静态亮度梯度），每个方块各自
   随机闪烁，像星空。没有从右往左的点亮波前，也没有扫描高光。

   用 canvas 2D（不是 WebGL）：2D context 任何机器/显卡都支持，不会像
   之前的 WebGL2 火焰那样在弱显卡/无 WebGL2 上整条黑掉（见 product-for-mass-users）。
   动画常驻运行，亮度差异交给 CSS 的 data-active：idle 压暗一档、active 全亮。 */

const ROWS = 6;

type CubeEngine = { destroy: () => void };

function createCubeGridEngine(canvas: HTMLCanvasElement): CubeEngine | null {
  const ctx = canvas.getContext("2d");
  if (!ctx) return null;

  let rafId: number | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let resizeDebounce: number | null = null;
  let cols = 0;
  let cell = 0;
  let hashes = new Float32Array(0);
  const startT = performance.now();
  // Claude 思考/ultracode 那种紫色（偏深，#7c3aed 系），而非全局偏蓝的 --ops-primary-rgb。
  // 亮块向近白推，暗块保持这个深紫核。
  let rgb: [number, number, number] = [124, 58, 237];

  function readColor() {
    // 优先取活动条自定义紫；无则退回默认紫（不再取偏蓝的 --ops-primary-rgb）。
    const v = getComputedStyle(canvas).getPropertyValue("--memory-wave-rgb").trim();
    if (!v) return;
    const p = v.split(/[\s,/]+/).map(Number).filter((n) => !Number.isNaN(n));
    if (p.length >= 3) rgb = [p[0], p[1], p[2]];
  }

  function resize() {
    const rect = canvas.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(rect.width * dpr));
    canvas.height = Math.max(1, Math.round(rect.height * dpr));
    cell = canvas.height / ROWS;
    cols = Math.max(1, Math.floor(canvas.width / cell));
    hashes = new Float32Array(cols * ROWS);
    for (let i = 0; i < hashes.length; i++) hashes[i] = Math.random();
    readColor();
  }

  function render(now: number) {
    if (document.hidden) {
      rafId = requestAnimationFrame(render);
      return;
    }
    const t = (now - startT) / 1000;
    ctx!.clearRect(0, 0, canvas.width, canvas.height);

    const pad = cell * 0.18;
    const size = Math.max(1, cell - pad * 2);

    for (let c = 0; c < cols; c++) {
      const cx = cols > 1 ? c / (cols - 1) : 1; // 0=最左 1=最右
      // 静态亮度梯度：右侧(cx→1)完全点亮，向左(cx→0)逐渐灰暗到近乎透明。
      // 曲线要陡（指数大）、底要低，才能让左侧三分之一真正暗下去。
      const level = 0.04 + Math.pow(cx, 2.4) * 0.96;
      const x = c * cell + pad;

      for (let r = 0; r < ROWS; r++) {
        const h = hashes[c * ROWS + r];
        // 每个方块各自随机闪烁，像星空；闪烁强度也随 level 缩放，暗列不被抬亮。
        const flicker = 0.5 + 0.5 * Math.sin(t * (3 + h * 4) + h * 6.283);
        let b = level * (0.55 + flicker * 0.6);
        b = Math.min(b, 1.1);
        // 亮度高时向近白推，低时保持深紫核。
        const mix = Math.max(0, Math.min((b - 0.6) / 0.5, 1));
        const rr = (rgb[0] + (255 - rgb[0]) * mix) | 0;
        const gg = (rgb[1] + (255 - rgb[1]) * mix) | 0;
        const bb = (rgb[2] + (255 - rgb[2]) * mix) | 0;
        ctx!.fillStyle = `rgba(${rr},${gg},${bb},${Math.min(b, 1)})`;
        ctx!.fillRect(x, r * cell + pad, size, size);
      }
    }
    rafId = requestAnimationFrame(render);
  }

  resizeObserver = new ResizeObserver(() => {
    if (resizeDebounce) clearTimeout(resizeDebounce);
    resizeDebounce = window.setTimeout(resize, 80);
  });
  resizeObserver.observe(canvas);
  resize();
  rafId = requestAnimationFrame(render);

  return {
    destroy() {
      if (rafId) cancelAnimationFrame(rafId);
      if (resizeObserver) resizeObserver.disconnect();
      if (resizeDebounce) clearTimeout(resizeDebounce);
    },
  };
}

export function MemoryActivityWave({ active }: { active: boolean }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const engine = createCubeGridEngine(canvas);
    return () => engine?.destroy();
  }, []);

  return (
    <span className="memory-activity-wave" data-active={active} aria-hidden="true">
      <canvas ref={canvasRef} />
    </span>
  );
}
