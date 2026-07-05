import { useEffect, useRef } from "react";

/* 对话监控活动条：WebGL2 四段管线火焰。
   移植自 254558/claude-range-slider 的 useWebglFire + shaders。
   火焰噪声天生无周期，会真实忽明忽暗地闪烁并从左往右流动，
   而不是 radial-gradient 圆点网格（必生 moiré 斜纹）或平移噪声（只流不闪）。
   本条恒定「满槽」（u_slider≈1），active 时跑渲染循环，idle 时淡出并停循环。 */

const VERT = `#version 300 es
  layout(location=0) in vec2 a_pos;
  out vec2 v_uv;
  void main(){ v_uv=a_pos*0.5+0.5; gl_Position=vec4(a_pos,0.0,1.0); }
`;

const FRAG_SIM = `#version 300 es
  precision highp float;
  in vec2 v_uv; out vec4 fc;
  uniform float u_time, u_slider, u_elapsed;
  uniform sampler2D u_back;
  float hash(vec2 p){ return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453); }
  void main(){
    vec2 uv=v_uv;
    vec2 g=uv*vec2(72.0,6.0);
    vec2 id=floor(g);
    vec2 cf=fract(g);
    float h=hash(id);
    vec2 ap=abs(cf-0.5);
    float cell=smoothstep(0.34,0.22,max(ap.x*0.9,ap.y));
    vec3 prev=texture(u_back,uv).rgb;
    float fade_mask = smoothstep(0.0, 0.22, uv.x);
    vec3 decay = prev * 0.90 * fade_mask;
    float act=smoothstep(0.95,1.0,u_slider);
    if(act<0.01||u_elapsed<0.0){ fc=vec4(decay,1.0); return; }
    float t=u_time;
    float cellDelay = h * 1.2;
    float cellAge   = max(u_elapsed - cellDelay, 0.0);
    float ignited   = step(0.001, cellAge);
    float cellSpd   = 0.85 + h * 0.30;
    float eased = 1.0 - pow(1.0 - clamp(cellAge / 2.5, 0.0, 1.0), 3.0);
    float dist  = eased * u_slider * cellSpd * ignited;
    float cellOff = (h - 0.5) * 0.05;
    float front   = max(u_slider - dist - cellOff, 0.02);
    float tail    = max(u_slider - front, 0.001);
    float inZ   = step(front - 0.003, uv.x) * step(uv.x, u_slider + 0.003);
    float dn    = clamp(max(u_slider - uv.x, 0.0) / tail, 0.0, 1.0);
    float bright = pow(1.0 - dn, 0.65);
    bright = max(bright, 0.04 * ignited) * inZ;
    bright *= 1.0 - smoothstep(0.94, 1.05, dn);
    float es = mix(0.15, 0.5, min(u_elapsed / 1.0, 1.0));
    float vy = abs(uv.y - 0.5) * 2.0;
    float vf = pow(max(1.0 - vy * vy * 0.45, 0.0), 0.75);
    float ts = mix(0.85, 1.0, min(u_elapsed / 1.5, 1.0));
    float f1 = sin(uv.x * 30.0 + t * 15.0 * ts + h * 6.28);
    float f2 = sin(uv.x * 17.0 + t * 8.0 * ts + h * 3.14);
    float f3 = sin(uv.x * 52.0 + t * 25.0 * ts + h * 10.0);
    float flame = smoothstep(0.08, 0.92, (f1 + f2 * 0.5 + f3 * 0.25) * 0.35 + 0.5);
    float r1 = sin(dn * 16.0 - t * 5.0 * ts + h * 3.0);
    float r2 = sin(dn * 8.0 - t * 2.5 * ts + h * 5.0);
    float rhythm = smoothstep(-0.15, 0.55, r1) * (r2 * 0.5 + 0.5);
    rhythm = pow(max(rhythm, 0.0), 1.2);
    float avgSpd = dist / max(cellAge, 0.001);
    float age    = max(cellAge - max(u_slider - uv.x, 0.0) / max(avgSpd, 0.001), 0.0);
    float flash  = step(0.0, age) * exp(-age * 3.2);
    float sp  = fract(t * (0.38 + h * 0.15) + h * 7.0);
    float sX  = u_slider - sp * tail;
    float sY  = 0.5 + sin(sp * 11.0 + h * 6.28) * 0.28;
    float spark = smoothstep(0.014, 0.0, abs(uv.x - sX))
                * smoothstep(0.18, 0.0, abs(uv.y - sY))
                * (1.0 - sp) * (1.0 - sp) * es;
    float energy = bright * vf * (flame * 0.42 + rhythm * 0.38)
                 + flash * bright * vf * 0.55
                 + spark * 0.7 * inZ;
    energy *= es;
    float edgeBase = exp(-pow((uv.x - front) * 18.0, 2.0));
    float ef1 = sin(uv.x * 45.0 + t * 20.0 * ts + h * 6.28) * 0.5 + 0.5;
    float ef2 = sin(uv.x * 28.0 + t * 11.0 * ts + h * 3.14) * 0.5 + 0.5;
    float edge = edgeBase * (0.25 + ef1 * ef2 * 1.5) * 1.6 * act * es;
    float leadD    = front - uv.x;
    float leadZone = smoothstep(0.07, 0.0, leadD) * step(0.0, leadD) * vf;
    float h2       = hash(id + vec2(99.0, 33.0));
    float leadF    = sin(leadD * 100.0 + t * 20.0 * ts + h2 * 6.28) * 0.5 + 0.5;
    float leadSpark = leadZone * step(0.6, h2) * leadF * act * es * 0.5;
    float total = energy + edge + leadSpark;
    vec3 ember = vec3(0.28, 0.10, 0.58);
    vec3 wpur  = vec3(0.62, 0.32, 1.0);
    vec3 wht   = vec3(1.0, 0.94, 0.98);
    float temp = 1.0 - dn;
    vec3 col   = mix(ember, wpur, temp);
    col        = mix(col, wht, pow(temp, 4.5));
    col       *= total;
    float pulse = sin(t * 2.8) * 0.15 + 1.0;
    float core  = exp(-pow((uv.x - u_slider) * 16.0, 2.0));
    col += wht * core * 2.2 * pulse * act * es;
    col += wpur * exp(-pow((uv.x - u_slider) * 3.5, 2.0)) * 0.12 * act * es;
    col *= cell;
    col *= fade_mask;
    fc = vec4(min(decay + col, vec3(1.5)), 1.0);
  }
`;

const FRAG_BLUR = `#version 300 es
  precision highp float;
  in vec2 v_uv; out vec4 fc;
  uniform sampler2D u_tex;
  uniform vec2 u_dir, u_res;
  uniform float u_ext;
  vec3 s(vec2 uv){
    vec3 c=texture(u_tex,uv).rgb;
    return u_ext>0.5 && dot(c,vec3(0.2126,0.7152,0.0722))<0.3 ? vec3(0.0) : c;
  }
  void main(){
    vec2 o=u_dir*1.8/u_res;
    vec3 r=s(v_uv)*0.227027;
    r+=s(v_uv+o)*0.194595;    r+=s(v_uv-o)*0.194595;
    r+=s(v_uv+o*2.0)*0.121622;r+=s(v_uv-o*2.0)*0.121622;
    r+=s(v_uv+o*3.0)*0.054054;r+=s(v_uv-o*3.0)*0.054054;
    fc=vec4(r,1.0);
  }
`;

const FRAG_COMP = `#version 300 es
  precision highp float;
  in vec2 v_uv; out vec4 fc;
  uniform sampler2D u_scene, u_glow;
  void main(){
    vec3 s=texture(u_scene,v_uv).rgb;
    vec3 g=texture(u_glow,v_uv).rgb;
    fc=vec4(1.0-exp(-(s+g*1.2+s*g*0.35)*1.15),1.0);
  }
`;

const MAX_IDLE = 180;
const SLIDER = 1.0;

type FboEntry = { fbo: WebGLFramebuffer; tex: WebGLTexture } | null;

type FireEngine = {
  setActive: (active: boolean) => void;
  destroy: () => void;
};

function createFireEngine(canvas: HTMLCanvasElement): FireEngine | null {
  const gl = canvas.getContext("webgl2", { preserveDrawingBuffer: false, antialias: false });
  if (!gl) return null;

  let rafId: number | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let resizeDebounce: number | null = null;
  let loopRunning = false;
  let idleFrames = 0;
  let wasActive = false;
  let ultraStart: number | null = null;
  let cachedActive = false;

  let simProg: WebGLProgram | null = null;
  let blurProg: WebGLProgram | null = null;
  let compProg: WebGLProgram | null = null;
  let vao: WebGLVertexArrayObject | null = null;
  let vbo: WebGLBuffer | null = null;
  let programsReady = false;

  let simA: FboEntry = null;
  let simB: FboEntry = null;
  let blurH: FboEntry = null;
  let blurV: FboEntry = null;

  const U: Record<string, WebGLUniformLocation | null> = {};

  function compileShader(type: number, src: string) {
    const sh = gl!.createShader(type)!;
    gl!.shaderSource(sh, src);
    gl!.compileShader(sh);
    if (!gl!.getShaderParameter(sh, gl!.COMPILE_STATUS)) {
      console.error(gl!.getShaderInfoLog(sh));
      gl!.deleteShader(sh);
      return null;
    }
    return sh;
  }

  function linkProgram(vsSrc: string, fsSrc: string) {
    const v = compileShader(gl!.VERTEX_SHADER, vsSrc);
    const f = compileShader(gl!.FRAGMENT_SHADER, fsSrc);
    if (!v || !f) return null;
    const p = gl!.createProgram()!;
    gl!.attachShader(p, v);
    gl!.attachShader(p, f);
    gl!.bindAttribLocation(p, 0, "a_pos");
    gl!.linkProgram(p);
    gl!.deleteShader(v);
    gl!.deleteShader(f);
    if (!gl!.getProgramParameter(p, gl!.LINK_STATUS)) {
      console.error(gl!.getProgramInfoLog(p));
      return null;
    }
    return p;
  }

  function compilePrograms() {
    simProg = linkProgram(VERT, FRAG_SIM);
    blurProg = linkProgram(VERT, FRAG_BLUR);
    compProg = linkProgram(VERT, FRAG_COMP);
    if (!simProg || !blurProg || !compProg) return;

    vao = gl!.createVertexArray();
    gl!.bindVertexArray(vao);
    vbo = gl!.createBuffer();
    gl!.bindBuffer(gl!.ARRAY_BUFFER, vbo);
    gl!.bufferData(gl!.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]), gl!.STATIC_DRAW);
    gl!.enableVertexAttribArray(0);
    gl!.vertexAttribPointer(0, 2, gl!.FLOAT, false, 0, 0);

    U.simTime = gl!.getUniformLocation(simProg, "u_time");
    U.simSlider = gl!.getUniformLocation(simProg, "u_slider");
    U.simElapsed = gl!.getUniformLocation(simProg, "u_elapsed");
    U.simBack = gl!.getUniformLocation(simProg, "u_back");
    U.blurDir = gl!.getUniformLocation(blurProg, "u_dir");
    U.blurExt = gl!.getUniformLocation(blurProg, "u_ext");
    U.blurTex = gl!.getUniformLocation(blurProg, "u_tex");
    U.blurRes = gl!.getUniformLocation(blurProg, "u_res");
    U.compScene = gl!.getUniformLocation(compProg, "u_scene");
    U.compGlow = gl!.getUniformLocation(compProg, "u_glow");
    programsReady = true;
  }

  function makeFBO(): FboEntry {
    const fbo = gl!.createFramebuffer()!;
    const tex = gl!.createTexture()!;
    gl!.bindFramebuffer(gl!.FRAMEBUFFER, fbo);
    gl!.bindTexture(gl!.TEXTURE_2D, tex);
    gl!.texImage2D(gl!.TEXTURE_2D, 0, gl!.RGBA, canvas.width, canvas.height, 0, gl!.RGBA, gl!.UNSIGNED_BYTE, null);
    gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_MIN_FILTER, gl!.LINEAR);
    gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_MAG_FILTER, gl!.LINEAR);
    gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_WRAP_S, gl!.CLAMP_TO_EDGE);
    gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_WRAP_T, gl!.CLAMP_TO_EDGE);
    gl!.framebufferTexture2D(gl!.FRAMEBUFFER, gl!.COLOR_ATTACHMENT0, gl!.TEXTURE_2D, tex, 0);
    gl!.clearColor(0, 0, 0, 1);
    gl!.clear(gl!.COLOR_BUFFER_BIT);
    return { fbo, tex };
  }

  function destroyFBO(entry: FboEntry) {
    if (!entry) return;
    gl!.deleteFramebuffer(entry.fbo);
    gl!.deleteTexture(entry.tex);
  }

  function destroyFBOs() {
    destroyFBO(simA); simA = null;
    destroyFBO(simB); simB = null;
    destroyFBO(blurH); blurH = null;
    destroyFBO(blurV); blurV = null;
  }

  function createFBOs() {
    simA = makeFBO();
    simB = makeFBO();
    blurH = makeFBO();
    blurV = makeFBO();
  }

  function resize() {
    const rect = canvas.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(rect.width * dpr));
    canvas.height = Math.max(1, Math.round(rect.height * dpr));
    destroyFBOs();
    createFBOs();
  }

  function ensureLoop() {
    if (!simA || !simB) {
      resize();
      if (!simA || !simB) return;
    }
    if (loopRunning) { idleFrames = 0; return; }
    loopRunning = true;
    idleFrames = 0;
    wasActive = false;
    gl!.bindFramebuffer(gl!.FRAMEBUFFER, simA.fbo);
    gl!.clear(gl!.COLOR_BUFFER_BIT);
    gl!.bindFramebuffer(gl!.FRAMEBUFFER, simB.fbo);
    gl!.clear(gl!.COLOR_BUFFER_BIT);
    rafId = requestAnimationFrame(render);
  }

  function render(t: number) {
    const active = cachedActive;
    if (!active && !wasActive) {
      if (++idleFrames > MAX_IDLE) {
        loopRunning = false;
        rafId = null;
        return;
      }
      rafId = requestAnimationFrame(render);
      return;
    }
    idleFrames = 0;
    if (active && !wasActive) {
      gl!.bindFramebuffer(gl!.FRAMEBUFFER, simA!.fbo);
      gl!.clear(gl!.COLOR_BUFFER_BIT);
      gl!.bindFramebuffer(gl!.FRAMEBUFFER, simB!.fbo);
      gl!.clear(gl!.COLOR_BUFFER_BIT);
    }
    wasActive = active;

    const elapsed = active ? (performance.now() - (ultraStart || 0)) / 1000 : -1.0;

    gl!.viewport(0, 0, canvas.width, canvas.height);

    // pass 1: simulation
    gl!.bindFramebuffer(gl!.FRAMEBUFFER, simB!.fbo);
    gl!.useProgram(simProg);
    gl!.uniform1f(U.simTime, t * 0.001);
    gl!.uniform1f(U.simSlider, SLIDER);
    gl!.uniform1f(U.simElapsed, elapsed);
    gl!.activeTexture(gl!.TEXTURE0);
    gl!.bindTexture(gl!.TEXTURE_2D, simA!.tex);
    gl!.uniform1i(U.simBack, 0);
    gl!.drawArrays(gl!.TRIANGLES, 0, 6);

    // pass 2: horizontal blur
    gl!.useProgram(blurProg);
    gl!.uniform2f(U.blurRes, canvas.width, canvas.height);
    gl!.bindFramebuffer(gl!.FRAMEBUFFER, blurH!.fbo);
    gl!.uniform2f(U.blurDir, 1.0, 0.0);
    gl!.uniform1f(U.blurExt, 1.0);
    gl!.bindTexture(gl!.TEXTURE_2D, simB!.tex);
    gl!.uniform1i(U.blurTex, 0);
    gl!.drawArrays(gl!.TRIANGLES, 0, 6);

    // pass 3: vertical blur
    gl!.bindFramebuffer(gl!.FRAMEBUFFER, blurV!.fbo);
    gl!.uniform2f(U.blurDir, 0.0, 1.0);
    gl!.uniform1f(U.blurExt, 0.0);
    gl!.bindTexture(gl!.TEXTURE_2D, blurH!.tex);
    gl!.drawArrays(gl!.TRIANGLES, 0, 6);

    // pass 4: composite to screen
    gl!.bindFramebuffer(gl!.FRAMEBUFFER, null);
    gl!.useProgram(compProg);
    gl!.activeTexture(gl!.TEXTURE0);
    gl!.bindTexture(gl!.TEXTURE_2D, simB!.tex);
    gl!.uniform1i(U.compScene, 0);
    gl!.activeTexture(gl!.TEXTURE1);
    gl!.bindTexture(gl!.TEXTURE_2D, blurV!.tex);
    gl!.uniform1i(U.compGlow, 1);
    gl!.drawArrays(gl!.TRIANGLES, 0, 6);

    const tmp = simA; simA = simB; simB = tmp;
    rafId = requestAnimationFrame(render);
  }

  compilePrograms();
  if (!programsReady) return null;

  resizeObserver = new ResizeObserver(() => {
    if (resizeDebounce) clearTimeout(resizeDebounce);
    resizeDebounce = window.setTimeout(resize, 80);
  });
  resizeObserver.observe(canvas);
  resize();

  return {
    setActive(active: boolean) {
      cachedActive = active;
      if (active) {
        if (ultraStart == null) ultraStart = performance.now();
        ensureLoop();
      } else {
        ultraStart = null;
      }
    },
    destroy() {
      if (rafId) { cancelAnimationFrame(rafId); rafId = null; }
      if (resizeObserver) { resizeObserver.disconnect(); resizeObserver = null; }
      if (resizeDebounce) { clearTimeout(resizeDebounce); resizeDebounce = null; }
      loopRunning = false;
      destroyFBOs();
      if (simProg) { gl!.deleteProgram(simProg); simProg = null; }
      if (blurProg) { gl!.deleteProgram(blurProg); blurProg = null; }
      if (compProg) { gl!.deleteProgram(compProg); compProg = null; }
      if (vao) { gl!.deleteVertexArray(vao); vao = null; }
      if (vbo) { gl!.deleteBuffer(vbo); vbo = null; }
      programsReady = false;
    },
  };
}

export function MemoryActivityWave({ active }: { active: boolean }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const engineRef = useRef<FireEngine | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const engine = createFireEngine(canvas);
    engineRef.current = engine;
    return () => {
      engine?.destroy();
      engineRef.current = null;
    };
  }, []);

  // 火焰常驻燃烧（active/idle 都点着），亮度差异交给 CSS 的 data-active 控制。
  useEffect(() => {
    engineRef.current?.setActive(true);
  }, []);

  return (
    <span className="memory-activity-wave" data-active={active} aria-hidden="true">
      <canvas ref={canvasRef} />
    </span>
  );
}
