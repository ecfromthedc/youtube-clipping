var Za=Object.defineProperty;var dr=(e,t,n)=>()=>{if(n)throw n[0];try{return e&&(t=e(e=0)),t}catch(r){throw n=[r],r}};var hr=(e,t)=>{for(var n in t)Za(e,n,{get:t[n],enumerable:!0})};function ha(e,t,n,r){let o=Math.max(1,Math.min(e,t)),i=Math.min(n,20),a=Math.min(i+r,o),u=Math.min(a,Math.floor(e/2)),l=Math.min(a,Math.floor(t/2)),p=_=>_/e*2-1,m=_=>_/t*2-1,d=0,f=e,g=0,O=t,w=u,C=e-u,P=l,A=t-l,b=p(d),k=p(f),W=m(g),R=m(O),fe=p(w),se=p(C),te=m(P),Y=m(A),ne=0,Xe=0,me=1,Ke=1,Qe=u/e,et=1-u/e,ge=l/t,re=1-l/t,qt=new Float32Array([b,W,k,W,b,te,b,te,k,W,k,te,b,Y,k,Y,b,R,b,R,k,Y,k,R,b,te,fe,te,b,Y,b,Y,fe,te,fe,Y,se,te,k,te,se,Y,se,Y,k,te,k,Y]),c=new Float32Array([ne,Xe,me,Xe,ne,ge,ne,ge,me,Xe,me,ge,ne,re,me,re,ne,Ke,ne,Ke,me,re,me,Ke,ne,ge,Qe,ge,ne,re,ne,re,Qe,ge,Qe,re,et,ge,me,ge,et,re,et,re,me,ge,me,re]);return{positions:qt,uvs:c}}function fa(e,t,n){let r=e.createShader(t);if(!r)throw new Error("Failed to create shader");if(e.shaderSource(r,n),e.compileShader(r),!e.getShaderParameter(r,e.COMPILE_STATUS)){let o=e.getShaderInfoLog(r)||"Unknown shader error";throw e.deleteShader(r),new Error(o)}return r}function Xu(e,t,n){let r=fa(e,e.VERTEX_SHADER,t),o=fa(e,e.FRAGMENT_SHADER,n),i=e.createProgram();if(!i)throw new Error("Failed to create program");if(e.attachShader(i,r),e.attachShader(i,o),e.linkProgram(i),!e.getProgramParameter(i,e.LINK_STATUS)){let s=e.getProgramInfoLog(i)||"Unknown link error";throw e.deleteProgram(i),e.deleteShader(r),e.deleteShader(o),new Error(s)}return e.deleteShader(r),e.deleteShader(o),i}function tl(e){let t=e.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);if(!t)throw new Error(`Invalid color format: ${e}`);let[,n,r,o]=t;return[parseInt(n)/255,parseInt(r)/255,parseInt(o)/255]}var Ku,Qu,el,Ht,ma=dr(()=>{Ku=`#version 300 es
precision lowp float;
in vec2 vUV;
out vec4 outColor;
uniform vec2 uResolution;
uniform float uTime;
uniform float uBorderWidth;
uniform float uGlowWidth;
uniform float uBorderRadius;
uniform vec3 uColors[4];
uniform float uGlowExponent;
uniform float uGlowFactor;
const float PI = 3.14159265359;
const float TWO_PI = 2.0 * PI;
const float HALF_PI = 0.5 * PI;
const vec4 startPositions = vec4(0.0, PI, HALF_PI, 1.5 * PI);
const vec4 speeds = vec4(-1.9, -1.9, -1.5, 2.1);
const vec4 innerRadius = vec4(PI * 0.8, PI * 0.7, PI * 0.3, PI * 0.1);
const vec4 outerRadius = vec4(PI * 1.2, PI * 0.9, PI * 0.6, PI * 0.4);
float random(vec2 st) {
return fract(sin(dot(st.xy, vec2(12.9898, 78.233))) * 43758.5453123);
}
vec2 random2(vec2 st) {
return vec2(random(st), random(st + 1.0));
}
float aaStep(float edge, float d) {
float width = fwidth(d);
return smoothstep(edge - width * 0.5, edge + width * 0.5, d);
}
float aaFract(float x) {
float f = fract(x);
float w = fwidth(x);
float smooth_f = f * (1.0 - smoothstep(1.0 - w, 1.0, f));
return smooth_f;
}
float sdRoundedBox(in vec2 p, in vec2 b, in float r) {
vec2 q = abs(p) - b + r;
return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}
float getInnerGlow(vec2 p, vec2 b, float radius) {
float dist_x = b.x - abs(p.x);
float dist_y = b.y - abs(p.y);
float glow_x = smoothstep(radius, 0.0, dist_x);
float glow_y = smoothstep(radius, 0.0, dist_y);
return 1.0 - (1.0 - glow_x) * (1.0 - glow_y);
}
float getVignette(vec2 uv) {
vec2 vignetteUv = uv;
vignetteUv = vignetteUv * (1.0 - vignetteUv);
float vignette = vignetteUv.x * vignetteUv.y * 25.0;
vignette = pow(vignette, 0.16);
vignette = 1.0 - vignette;
return vignette;
}
float uvToAngle(vec2 uv) {
vec2 center = vec2(0.5);
vec2 dir = uv - center;
return atan(dir.y, dir.x) + PI;
}
void main() {
vec2 uv = vUV;
vec2 pos = uv * uResolution;
vec2 centeredPos = pos - uResolution * 0.5;
vec2 size = uResolution - uBorderWidth;
vec2 halfSize = size * 0.5;
float dBorderBox = sdRoundedBox(centeredPos, halfSize, uBorderRadius);
float border = aaStep(0.0, dBorderBox);
float glow = getInnerGlow(centeredPos, halfSize, uGlowWidth);
float vignette = getVignette(uv);
glow *= vignette;
float posAngle = uvToAngle(uv);
vec4 lightCenter = mod(startPositions + speeds * uTime, TWO_PI);
vec4 angleDist = abs(posAngle - lightCenter);
vec4 disToLight = min(angleDist, TWO_PI - angleDist) / TWO_PI;
float intensityBorder[4];
intensityBorder[0] = 1.0;
intensityBorder[1] = smoothstep(0.4, 0.0, disToLight.y);
intensityBorder[2] = smoothstep(0.4, 0.0, disToLight.z);
intensityBorder[3] = smoothstep(0.2, 0.0, disToLight.w) * 0.5;
vec3 borderColor = vec3(0.0);
for(int i = 0; i < 4; i++) {
borderColor = mix(borderColor, uColors[i], intensityBorder[i]);
}
borderColor *= 1.1;
borderColor = clamp(borderColor, 0.0, 1.0);
float intensityGlow[4];
intensityGlow[0] = smoothstep(0.9, 0.0, disToLight.x);
intensityGlow[1] = smoothstep(0.7, 0.0, disToLight.y);
intensityGlow[2] = smoothstep(0.4, 0.0, disToLight.z);
intensityGlow[3] = smoothstep(0.1, 0.0, disToLight.w) * 0.7;
vec4 breath = smoothstep(0.0, 1.0, sin(uTime * 1.0 + startPositions * PI) * 0.2 + 0.8);
vec3 glowColor = vec3(0.0);
glowColor += uColors[0] * intensityGlow[0] * breath.x;
glowColor += uColors[1] * intensityGlow[1] * breath.y;
glowColor += uColors[2] * intensityGlow[2] * breath.z;
glowColor += uColors[3] * intensityGlow[3] * breath.w * glow;
glow = pow(glow, uGlowExponent);
glow *= random(pos + uTime) * 0.1 + 1.0;
glowColor *= glow * uGlowFactor;
glowColor = clamp(glowColor, 0.0, 1.0);
vec3 color = mix(glowColor, borderColor + glowColor * 0.2, border);
float alpha = mix(glow, 1.0, border);
outColor = vec4(color, alpha);
}`,Qu=`#version 300 es
in vec2 aPosition;
in vec2 aUV;
out vec2 vUV;
void main() {
vUV = aUV;
gl_Position = vec4(aPosition, 0.0, 1.0);
}`;el=["rgb(57, 182, 255)","rgb(189, 69, 251)","rgb(255, 87, 51)","rgb(255, 214, 0)"];Ht=class{element;canvas;options;running=!1;disposed=!1;startTime=0;lastTime=0;rafId=null;glr;observer;constructor(t={}){this.options={width:t.width??600,height:t.height??600,ratio:t.ratio??window.devicePixelRatio??1,borderWidth:t.borderWidth??8,glowWidth:t.glowWidth??200,borderRadius:t.borderRadius??8,mode:t.mode??"light",...t},this.canvas=document.createElement("canvas"),this.options.classNames&&(this.canvas.className=this.options.classNames),this.options.styles&&Object.assign(this.canvas.style,this.options.styles),this.canvas.style.display="block",this.canvas.style.transformOrigin="center",this.canvas.style.pointerEvents="none",this.element=this.canvas,this.setupGL(),this.options.skipGreeting||this.greet()}start(){if(this.disposed)throw new Error("Motion instance has been disposed.");if(this.running)return;if(!this.glr){console.error("WebGL resources are not initialized.");return}this.running=!0,this.startTime=performance.now(),this.resize(this.options.width??600,this.options.height??600,this.options.ratio),this.glr.gl.viewport(0,0,this.canvas.width,this.canvas.height),this.glr.gl.useProgram(this.glr.program),this.glr.gl.uniform2f(this.glr.uResolution,this.canvas.width,this.canvas.height),this.checkGLError(this.glr.gl,"start: after initial setup");let t=()=>{if(!this.running||!this.glr)return;this.rafId=requestAnimationFrame(t);let n=performance.now();if(n-this.lastTime<1e3/32)return;this.lastTime=n;let o=(n-this.startTime)*.001;this.render(o)};this.rafId=requestAnimationFrame(t)}pause(){if(this.disposed)throw new Error("Motion instance has been disposed.");this.running=!1,this.rafId!==null&&cancelAnimationFrame(this.rafId)}dispose(){if(this.disposed)return;this.disposed=!0,this.running=!1,this.rafId!==null&&cancelAnimationFrame(this.rafId);let{gl:t,vao:n,positionBuffer:r,uvBuffer:o,program:i}=this.glr;n&&t.deleteVertexArray(n),r&&t.deleteBuffer(r),o&&t.deleteBuffer(o),t.deleteProgram(i),this.observer&&this.observer.disconnect(),this.canvas.remove()}resize(t,n,r){if(this.disposed)throw new Error("Motion instance has been disposed.");if(this.options.width=t,this.options.height=n,r&&(this.options.ratio=r),!this.running)return;let{gl:o,program:i,vao:s,positionBuffer:a,uvBuffer:u,uResolution:l}=this.glr,p=r??this.options.ratio??window.devicePixelRatio??1,m=Math.max(1,Math.floor(t*p)),d=Math.max(1,Math.floor(n*p));this.canvas.style.width=`${t}px`,this.canvas.style.height=`${n}px`,(this.canvas.width!==m||this.canvas.height!==d)&&(this.canvas.width=m,this.canvas.height=d),o.viewport(0,0,this.canvas.width,this.canvas.height),this.checkGLError(o,"resize: after viewport setup");let{positions:f,uvs:g}=ha(this.canvas.width,this.canvas.height,this.options.borderWidth*p,this.options.glowWidth*p);o.bindVertexArray(s),o.bindBuffer(o.ARRAY_BUFFER,a),o.bufferData(o.ARRAY_BUFFER,f,o.STATIC_DRAW);let O=o.getAttribLocation(i,"aPosition");o.enableVertexAttribArray(O),o.vertexAttribPointer(O,2,o.FLOAT,!1,0,0),this.checkGLError(o,"resize: after position buffer update"),o.bindBuffer(o.ARRAY_BUFFER,u),o.bufferData(o.ARRAY_BUFFER,g,o.STATIC_DRAW);let w=o.getAttribLocation(i,"aUV");o.enableVertexAttribArray(w),o.vertexAttribPointer(w,2,o.FLOAT,!1,0,0),this.checkGLError(o,"resize: after UV buffer update"),o.useProgram(i),o.uniform2f(l,this.canvas.width,this.canvas.height),o.uniform1f(this.glr.uBorderWidth,this.options.borderWidth*p),o.uniform1f(this.glr.uGlowWidth,this.options.glowWidth*p),o.uniform1f(this.glr.uBorderRadius,this.options.borderRadius*p),this.checkGLError(o,"resize: after uniform updates");let C=performance.now();this.lastTime=C;let P=(C-this.startTime)*.001;this.render(P)}autoResize(t){this.observer&&this.observer.disconnect(),this.observer=new ResizeObserver(()=>{let n=t.getBoundingClientRect();this.resize(n.width,n.height)}),this.observer.observe(t)}fadeIn(){if(this.disposed)throw new Error("Motion instance has been disposed.");return new Promise((t,n)=>{let r=this.canvas.animate([{opacity:0,transform:"scale(1.2)"},{opacity:1,transform:"scale(1)"}],{duration:300,easing:"ease-out",fill:"forwards"});r.onfinish=()=>t(),r.oncancel=()=>n("canceled")})}fadeOut(){if(this.disposed)throw new Error("Motion instance has been disposed.");return new Promise((t,n)=>{let r=this.canvas.animate([{opacity:1,transform:"scale(1)"},{opacity:0,transform:"scale(1.2)"}],{duration:300,easing:"ease-in",fill:"forwards"});r.onfinish=()=>t(),r.oncancel=()=>n("canceled")})}checkGLError(t,n){let r=t.getError();if(r!==t.NO_ERROR){for(console.group(`\u{1F534} WebGL Error in ${n}`);r!==t.NO_ERROR;){let o=this.getGLErrorName(t,r);console.error(`${o} (0x${r.toString(16)})`),r=t.getError()}console.groupEnd()}}getGLErrorName(t,n){switch(n){case t.INVALID_ENUM:return"INVALID_ENUM";case t.INVALID_VALUE:return"INVALID_VALUE";case t.INVALID_OPERATION:return"INVALID_OPERATION";case t.INVALID_FRAMEBUFFER_OPERATION:return"INVALID_FRAMEBUFFER_OPERATION";case t.OUT_OF_MEMORY:return"OUT_OF_MEMORY";case t.CONTEXT_LOST_WEBGL:return"CONTEXT_LOST_WEBGL";default:return"UNKNOWN_ERROR"}}setupGL(){let t=this.canvas.getContext("webgl2",{antialias:!1,alpha:!0});if(!t)throw new Error("WebGL2 is required but not available.");let n=Xu(t,Qu,Ku);this.checkGLError(t,"setupGL: after createProgram");let r=t.createVertexArray();t.bindVertexArray(r),this.checkGLError(t,"setupGL: after VAO creation");let o=this.canvas.width||2,i=this.canvas.height||2,{positions:s,uvs:a}=ha(o,i,this.options.borderWidth,this.options.glowWidth),u=t.createBuffer();t.bindBuffer(t.ARRAY_BUFFER,u),t.bufferData(t.ARRAY_BUFFER,s,t.STATIC_DRAW);let l=t.getAttribLocation(n,"aPosition");t.enableVertexAttribArray(l),t.vertexAttribPointer(l,2,t.FLOAT,!1,0,0),this.checkGLError(t,"setupGL: after position buffer setup");let p=t.createBuffer();t.bindBuffer(t.ARRAY_BUFFER,p),t.bufferData(t.ARRAY_BUFFER,a,t.STATIC_DRAW);let m=t.getAttribLocation(n,"aUV");t.enableVertexAttribArray(m),t.vertexAttribPointer(m,2,t.FLOAT,!1,0,0),this.checkGLError(t,"setupGL: after UV buffer setup");let d=t.getUniformLocation(n,"uResolution"),f=t.getUniformLocation(n,"uTime"),g=t.getUniformLocation(n,"uBorderWidth"),O=t.getUniformLocation(n,"uGlowWidth"),w=t.getUniformLocation(n,"uBorderRadius"),C=t.getUniformLocation(n,"uColors"),P=t.getUniformLocation(n,"uGlowExponent"),A=t.getUniformLocation(n,"uGlowFactor");t.useProgram(n),t.uniform1f(g,this.options.borderWidth),t.uniform1f(O,this.options.glowWidth),t.uniform1f(w,this.options.borderRadius),this.options.mode==="dark"?(t.uniform1f(P,2),t.uniform1f(A,1.8)):(t.uniform1f(P,1),t.uniform1f(A,1));let b=(this.options.colors||el).map(tl);for(let k=0;k<b.length;k++)t.uniform3f(t.getUniformLocation(n,`uColors[${k}]`),...b[k]);this.checkGLError(t,"setupGL: after uniform setup"),t.bindVertexArray(null),t.bindBuffer(t.ARRAY_BUFFER,null),this.glr={gl:t,program:n,vao:r,positionBuffer:u,uvBuffer:p,uResolution:d,uTime:f,uBorderWidth:g,uGlowWidth:O,uBorderRadius:w,uColors:C}}render(t){if(!this.glr)return;let{gl:n,program:r,vao:o,uTime:i}=this.glr;n.useProgram(r),n.bindVertexArray(o),n.uniform1f(i,t),n.disable(n.DEPTH_TEST),n.disable(n.CULL_FACE),n.disable(n.BLEND),n.clearColor(0,0,0,0),n.clear(n.COLOR_BUFFER_BIT),n.drawArrays(n.TRIANGLES,0,24),this.checkGLError(n,"render: after draw call"),n.bindVertexArray(null)}greet(){console.log("%c\u{1F308} ai-motion 0.4.8 \u{1F308}","background: linear-gradient(90deg, #39b6ff, #bd45fb, #ff5733, #ffd600); color: white; text-shadow: 0 0 2px rgba(0, 0, 0, 0.2); font-weight: bold; font-size: 1em; padding: 2px 12px; border-radius: 6px;")}}});var _a={};hr(_a,{SimulatorMask:()=>ll});function nl(){try{return!!(rl()||ol()||il()||sl()||cl()||al())}catch(e){return console.warn("Error determining if page is dark:",e),!1}}function rl(){let e=["dark","dark-mode","theme-dark","night","night-mode"],t=document.documentElement,n=document.body||document.documentElement;for(let r of e)if(t.classList.contains(r)||n?.classList.contains(r))return!0;return!1}function ol(){let e=document.documentElement,t=document.body||document.documentElement;for(let n of["data-theme","data-color-mode","data-bs-theme","data-mui-color-scheme"]){let r=t?.getAttribute(n),o=e.getAttribute(n);if(r?.toLowerCase()==="dark"||o?.toLowerCase()==="dark")return!0}return!1}function il(){let e=document.querySelector('meta[name="color-scheme"]')?.content.toLowerCase();if(e==="dark"||e==="only dark")return!0;let t=window.getComputedStyle(document.documentElement).getPropertyValue("color-scheme").trim().toLowerCase();return t==="dark"||t==="only dark"}function sl(){let e=window.getComputedStyle(document.documentElement),t=window.getComputedStyle(document.body||document.documentElement),n=e.backgroundColor,r=t.backgroundColor;return cr(r)?!0:r==="transparent"||r.startsWith("rgba(0, 0, 0, 0)")?cr(n):!1}function al(){let t=ga(window.getComputedStyle(document.body||document.documentElement).color);return t!==null&&t>200}function cl(){let{innerWidth:e,innerHeight:t}=window,n=e*t*.5;for(let r of["#app","#root","#__next"]){let o=document.querySelector(r);if(!o)continue;let i=o.getBoundingClientRect();if(!(i.width*i.height<n)&&cr(window.getComputedStyle(o).backgroundColor))return!0}return!1}function ul(e){let t=/rgba?\((\d+),\s*(\d+),\s*(\d+)/.exec(e);return t?{r:parseInt(t[1]),g:parseInt(t[2]),b:parseInt(t[3])}:null}function ga(e){if(!e||e==="transparent"||e.startsWith("rgba(0, 0, 0, 0)"))return null;let t=ul(e);return t?.299*t.r+.587*t.g+.114*t.b:null}function cr(e,t=128){let n=ga(e);return n!==null&&n<t}var ar,Le,ll,ba=dr(()=>{ma();(function(){try{if(typeof document<"u"){var e=document.createElement("style");e.appendChild(document.createTextNode(`._wrapper_1ooyb_1 {
	position: fixed;
	inset: 0;
	z-index: 2147483641; /* \u786E\u4FDD\u5728\u6240\u6709\u5143\u7D20\u4E4B\u4E0A\uFF0C\u9664\u4E86 panel */
	cursor: wait;
	overflow: hidden;

	display: none;
}

._wrapper_1ooyb_1._visible_1ooyb_11 {
	display: block;
}
/* AI \u5149\u6807\u6837\u5F0F */
._cursor_1dgwb_2 {
	position: absolute;
	width: var(--cursor-size, 75px);
	height: var(--cursor-size, 75px);
	pointer-events: none;
	z-index: 10000;
}

._cursorBorder_1dgwb_10 {
	position: absolute;
	width: 100%;
	height: 100%;
	background: linear-gradient(45deg, rgb(57, 182, 255), rgb(189, 69, 251));
	mask-image: url("data:image/svg+xml,%3csvg%20xmlns='http://www.w3.org/2000/svg'%20viewBox='0%200%20100%20100'%20fill='none'%3e%3cg%3e%3cpath%20d='M%2015%2042%20L%2015%2036.99%20Q%2015%2031.99%2023.7%2031.99%20L%2028.05%2031.99%20Q%2032.41%2031.99%2032.41%2021.99%20L%2032.41%2017%20Q%2032.41%2012%2041.09%2016.95%20L%2076.31%2037.05%20Q%2085%2042%2076.31%2046.95%20L%2041.09%2067.05%20Q%2032.41%2072%2032.41%2062.01%20L%2032.41%2057.01%20Q%2032.41%2052.01%2023.7%2052.01%20L%2019.35%2052.01%20Q%2015%2052.01%2015%2047.01%20Z'%20fill='none'%20stroke='%23000000'%20stroke-width='6'%20stroke-miterlimit='10'%20style='stroke:%20light-dark(rgb(0,%200,%200),%20rgb(255,%20255,%20255));'/%3e%3c/g%3e%3c/svg%3e");
	mask-size: 100% 100%;
	mask-repeat: no-repeat;

	transform-origin: center;
	transform: rotate(-135deg) scale(1.2);
	margin-left: -10px;
	margin-top: -18px;
}

._cursorFilling_1dgwb_25 {
	position: absolute;
	width: 100%;
	height: 100%;
	background: url("data:image/svg+xml,%3csvg%20xmlns='http://www.w3.org/2000/svg'%20viewBox='0%200%20100%20100'%3e%3cdefs%3e%3c/defs%3e%3cg%20xmlns='http://www.w3.org/2000/svg'%20style='filter:%20drop-shadow(light-dark(rgba(0,%200,%200,%200.4),%20rgba(237,%20237,%20237,%200.4))%203px%204px%204px);'%3e%3cpath%20d='M%2015%2042%20L%2015%2036.99%20Q%2015%2031.99%2023.7%2031.99%20L%2028.05%2031.99%20Q%2032.41%2031.99%2032.41%2021.99%20L%2032.41%2017%20Q%2032.41%2012%2041.09%2016.95%20L%2076.31%2037.05%20Q%2085%2042%2076.31%2046.95%20L%2041.09%2067.05%20Q%2032.41%2072%2032.41%2062.01%20L%2032.41%2057.01%20Q%2032.41%2052.01%2023.7%2052.01%20L%2019.35%2052.01%20Q%2015%2052.01%2015%2047.01%20Z'%20fill='%23ffffff'%20stroke='none'%20style='fill:%20%23ffffff;'/%3e%3c/g%3e%3c/svg%3e");
	background-size: 100% 100%;
	background-repeat: no-repeat;

	transform-origin: center;
	transform: rotate(-135deg) scale(1.2);
	margin-left: -10px;
	margin-top: -18px;
}

._cursorRipple_1dgwb_39 {
	position: absolute;
	width: 100%;
	height: 100%;
	pointer-events: none;
	margin-left: -50%;
	margin-top: -50%;

	&::after {
		content: '';
		opacity: 0;
		position: absolute;
		inset: 0;
		border: 4px solid rgba(57, 182, 255, 1);
		border-radius: 50%;
	}
}

._cursor_1dgwb_2._clicking_1dgwb_57 ._cursorRipple_1dgwb_39::after {
	animation: _cursor-ripple_1dgwb_1 300ms ease-out forwards;
}

@keyframes _cursor-ripple_1dgwb_1 {
	0% {
		transform: scale(0);
		opacity: 1;
	}
	100% {
		transform: scale(2);
		opacity: 0;
	}
}`)),document.head.appendChild(e)}}catch(t){console.error("vite-plugin-css-injected-by-js",t)}})();(function(){try{if(typeof document<"u"){var e=document.createElement("style");e.appendChild(document.createTextNode(`._wrapper_1ooyb_1 {
	position: fixed;
	inset: 0;
	z-index: 2147483641; /* \u786E\u4FDD\u5728\u6240\u6709\u5143\u7D20\u4E4B\u4E0A\uFF0C\u9664\u4E86 panel */
	cursor: wait;
	overflow: hidden;

	display: none;
}

._wrapper_1ooyb_1._visible_1ooyb_11 {
	display: block;
}
/* AI \u5149\u6807\u6837\u5F0F */
._cursor_1dgwb_2 {
	position: absolute;
	width: var(--cursor-size, 75px);
	height: var(--cursor-size, 75px);
	pointer-events: none;
	z-index: 10000;
}

._cursorBorder_1dgwb_10 {
	position: absolute;
	width: 100%;
	height: 100%;
	background: linear-gradient(45deg, rgb(57, 182, 255), rgb(189, 69, 251));
	mask-image: url("data:image/svg+xml,%3csvg%20xmlns='http://www.w3.org/2000/svg'%20viewBox='0%200%20100%20100'%20fill='none'%3e%3cg%3e%3cpath%20d='M%2015%2042%20L%2015%2036.99%20Q%2015%2031.99%2023.7%2031.99%20L%2028.05%2031.99%20Q%2032.41%2031.99%2032.41%2021.99%20L%2032.41%2017%20Q%2032.41%2012%2041.09%2016.95%20L%2076.31%2037.05%20Q%2085%2042%2076.31%2046.95%20L%2041.09%2067.05%20Q%2032.41%2072%2032.41%2062.01%20L%2032.41%2057.01%20Q%2032.41%2052.01%2023.7%2052.01%20L%2019.35%2052.01%20Q%2015%2052.01%2015%2047.01%20Z'%20fill='none'%20stroke='%23000000'%20stroke-width='6'%20stroke-miterlimit='10'%20style='stroke:%20light-dark(rgb(0,%200,%200),%20rgb(255,%20255,%20255));'/%3e%3c/g%3e%3c/svg%3e");
	mask-size: 100% 100%;
	mask-repeat: no-repeat;

	transform-origin: center;
	transform: rotate(-135deg) scale(1.2);
	margin-left: -10px;
	margin-top: -18px;
}

._cursorFilling_1dgwb_25 {
	position: absolute;
	width: 100%;
	height: 100%;
	background: url("data:image/svg+xml,%3csvg%20xmlns='http://www.w3.org/2000/svg'%20viewBox='0%200%20100%20100'%3e%3cdefs%3e%3c/defs%3e%3cg%20xmlns='http://www.w3.org/2000/svg'%20style='filter:%20drop-shadow(light-dark(rgba(0,%200,%200,%200.4),%20rgba(237,%20237,%20237,%200.4))%203px%204px%204px);'%3e%3cpath%20d='M%2015%2042%20L%2015%2036.99%20Q%2015%2031.99%2023.7%2031.99%20L%2028.05%2031.99%20Q%2032.41%2031.99%2032.41%2021.99%20L%2032.41%2017%20Q%2032.41%2012%2041.09%2016.95%20L%2076.31%2037.05%20Q%2085%2042%2076.31%2046.95%20L%2041.09%2067.05%20Q%2032.41%2072%2032.41%2062.01%20L%2032.41%2057.01%20Q%2032.41%2052.01%2023.7%2052.01%20L%2019.35%2052.01%20Q%2015%2052.01%2015%2047.01%20Z'%20fill='%23ffffff'%20stroke='none'%20style='fill:%20%23ffffff;'/%3e%3c/g%3e%3c/svg%3e");
	background-size: 100% 100%;
	background-repeat: no-repeat;

	transform-origin: center;
	transform: rotate(-135deg) scale(1.2);
	margin-left: -10px;
	margin-top: -18px;
}

._cursorRipple_1dgwb_39 {
	position: absolute;
	width: 100%;
	height: 100%;
	pointer-events: none;
	margin-left: -50%;
	margin-top: -50%;

	&::after {
		content: '';
		opacity: 0;
		position: absolute;
		inset: 0;
		border: 4px solid rgba(57, 182, 255, 1);
		border-radius: 50%;
	}
}

._cursor_1dgwb_2._clicking_1dgwb_57 ._cursorRipple_1dgwb_39::after {
	animation: _cursor-ripple_1dgwb_1 300ms ease-out forwards;
}

@keyframes _cursor-ripple_1dgwb_1 {
	0% {
		transform: scale(0);
		opacity: 1;
	}
	100% {
		transform: scale(2);
		opacity: 0;
	}
}`)),document.head.appendChild(e)}}catch(t){console.error("vite-plugin-css-injected-by-js",t)}})();ar={wrapper:"_wrapper_1ooyb_1",visible:"_visible_1ooyb_11"},Le={cursor:"_cursor_1dgwb_2",cursorBorder:"_cursorBorder_1dgwb_10",cursorFilling:"_cursorFilling_1dgwb_25",cursorRipple:"_cursorRipple_1dgwb_39",clicking:"_clicking_1dgwb_57","cursor-ripple":"_cursor-ripple_1dgwb_1"},ll=class extends EventTarget{shown=!1;wrapper=document.createElement("div");motion=null;#n=!1;#o=document.createElement("div");#t=0;#i=0;#c=0;#a=0;constructor(){super(),this.wrapper.id="page-agent-runtime_simulator-mask",this.wrapper.className=ar.wrapper,this.wrapper.setAttribute("data-browser-use-ignore","true"),this.wrapper.setAttribute("data-page-agent-ignore","true");try{let o=new Ht({mode:nl()?"dark":"light",styles:{position:"absolute",inset:"0"}});this.motion=o,this.wrapper.appendChild(o.element),o.autoResize(this.wrapper)}catch(o){console.warn("[SimulatorMask] Motion overlay unavailable:",o)}this.wrapper.addEventListener("click",o=>{o.stopPropagation(),o.preventDefault()}),this.wrapper.addEventListener("mousedown",o=>{o.stopPropagation(),o.preventDefault()}),this.wrapper.addEventListener("mouseup",o=>{o.stopPropagation(),o.preventDefault()}),this.wrapper.addEventListener("mousemove",o=>{o.stopPropagation(),o.preventDefault()}),this.wrapper.addEventListener("wheel",o=>{o.stopPropagation(),o.preventDefault()}),this.wrapper.addEventListener("keydown",o=>{o.stopPropagation(),o.preventDefault()}),this.wrapper.addEventListener("keyup",o=>{o.stopPropagation(),o.preventDefault()}),this.#s(),document.body.appendChild(this.wrapper),this.#l();let e=o=>{let{x:i,y:s}=o.detail;this.setCursorPosition(i,s)},t=()=>{this.triggerClickAnimation()},n=()=>{this.wrapper.style.pointerEvents="none"},r=()=>{this.wrapper.style.pointerEvents="auto"};window.addEventListener("PageAgent::MovePointerTo",e),window.addEventListener("PageAgent::ClickPointer",t),window.addEventListener("PageAgent::EnablePassThrough",n),window.addEventListener("PageAgent::DisablePassThrough",r),this.addEventListener("dispose",()=>{window.removeEventListener("PageAgent::MovePointerTo",e),window.removeEventListener("PageAgent::ClickPointer",t),window.removeEventListener("PageAgent::EnablePassThrough",n),window.removeEventListener("PageAgent::DisablePassThrough",r)})}#s(){this.#o.className=Le.cursor;let e=document.createElement("div");e.className=Le.cursorRipple,this.#o.appendChild(e);let t=document.createElement("div");t.className=Le.cursorFilling,this.#o.appendChild(t);let n=document.createElement("div");n.className=Le.cursorBorder,this.#o.appendChild(n),this.wrapper.appendChild(this.#o)}#l(){if(this.#n)return;let e=this.#t+(this.#c-this.#t)*.2,t=this.#i+(this.#a-this.#i)*.2,n=Math.abs(e-this.#c);n>0&&(n<2?this.#t=this.#c:this.#t=e,this.#o.style.left=`${this.#t}px`);let r=Math.abs(t-this.#a);r>0&&(r<2?this.#i=this.#a:this.#i=t,this.#o.style.top=`${this.#i}px`),requestAnimationFrame(()=>this.#l())}setCursorPosition(e,t){this.#n||(this.#c=e,this.#a=t)}triggerClickAnimation(){this.#n||(this.#o.classList.remove(Le.clicking),this.#o.offsetHeight,this.#o.classList.add(Le.clicking))}show(){this.shown||this.#n||(this.shown=!0,this.motion?.start(),this.motion?.fadeIn(),this.wrapper.classList.add(ar.visible),this.#t=window.innerWidth/2,this.#i=window.innerHeight/2,this.#c=this.#t,this.#a=this.#i,this.#o.style.left=`${this.#t}px`,this.#o.style.top=`${this.#i}px`)}hide(){!this.shown||this.#n||(this.shown=!1,this.motion?.fadeOut(),this.motion?.pause(),this.#o.classList.remove(Le.clicking),setTimeout(()=>{this.wrapper.classList.remove(ar.visible)},800))}dispose(){this.#n=!0,this.motion?.dispose(),this.wrapper.remove(),this.dispatchEvent(new Event("dispose"))}}});var fr;function h(e,t,n){function r(a,u){if(a._zod||Object.defineProperty(a,"_zod",{value:{def:u,constr:s,traits:new Set},enumerable:!1}),a._zod.traits.has(e))return;a._zod.traits.add(e),t(a,u);let l=s.prototype,p=Object.keys(l);for(let m=0;m<p.length;m++){let d=p[m];d in a||(a[d]=l[d].bind(a))}}let o=n?.Parent??Object;class i extends o{}Object.defineProperty(i,"name",{value:e});function s(a){var u;let l=n?.Parent?new i:this;r(l,a),(u=l._zod).deferred??(u.deferred=[]);for(let p of l._zod.deferred)p();return l}return Object.defineProperty(s,"init",{value:r}),Object.defineProperty(s,Symbol.hasInstance,{value:a=>n?.Parent&&a instanceof n.Parent?!0:a?._zod?.traits?.has(e)}),Object.defineProperty(s,"name",{value:e}),s}var xe=class extends Error{constructor(){super("Encountered Promise during synchronous parse. Use .parseAsync() instead.")}},je=class extends Error{constructor(t){super(`Encountered unidirectional transform during encode: ${t}`),this.name="ZodEncodeError"}};(fr=globalThis).__zod_globalConfig??(fr.__zod_globalConfig={});var Ue=globalThis.__zod_globalConfig;function le(e){return e&&Object.assign(Ue,e),Ue}var Z={};hr(Z,{BIGINT_FORMAT_RANGES:()=>br,Class:()=>Kt,NUMBER_FORMAT_RANGES:()=>on,aborted:()=>Te,allowsEval:()=>tn,assert:()=>Da,assertEqual:()=>Ra,assertIs:()=>Ma,assertNever:()=>Fa,assertNotEqual:()=>La,assignProp:()=>Se,base64ToUint8Array:()=>yr,base64urlToUint8Array:()=>rc,cached:()=>ot,captureStackTrace:()=>Pt,cleanEnum:()=>nc,cleanRegex:()=>st,clone:()=>pe,cloneDef:()=>Ua,createTransparentProxy:()=>Ha,defineLazy:()=>L,esc:()=>Tt,escapeRegex:()=>Ze,explicitlyAborted:()=>sn,extend:()=>Xa,finalizeIssue:()=>ye,floatSafeRemainder:()=>Qt,getElementAtPath:()=>Ba,getEnumValues:()=>rt,getLengthableOrigin:()=>ct,getParsedType:()=>Ja,getSizableOrigin:()=>xr,hexToUint8Array:()=>ic,isObject:()=>Be,isPlainObject:()=>Ce,issue:()=>Ve,joinValues:()=>St,jsonStringifyReplacer:()=>We,merge:()=>Qa,mergeDefs:()=>ke,normalizeParams:()=>z,nullish:()=>it,numKeys:()=>Ga,objectClone:()=>ja,omit:()=>Ya,optionalKeys:()=>rn,parsedType:()=>an,partial:()=>ec,pick:()=>qa,prefixIssues:()=>at,primitiveTypes:()=>_r,promiseAllObject:()=>Wa,propertyKeyTypes:()=>nn,randomString:()=>Va,required:()=>tc,safeExtend:()=>Ka,shallowClone:()=>gr,slugify:()=>en,stringifyPrimitive:()=>At,uint8ArrayToBase64:()=>wr,uint8ArrayToBase64url:()=>oc,uint8ArrayToHex:()=>sc,unwrapMessage:()=>nt});function Ra(e){return e}function La(e){return e}function Ma(e){}function Fa(e){throw new Error("Unexpected value in exhaustive check")}function Da(e){}function rt(e){let t=Object.values(e).filter(r=>typeof r=="number");return Object.entries(e).filter(([r,o])=>t.indexOf(+r)===-1).map(([r,o])=>o)}function St(e,t="|"){return e.map(n=>At(n)).join(t)}function We(e,t){return typeof t=="bigint"?t.toString():t}function ot(e){return{get value(){{let n=e();return Object.defineProperty(this,"value",{value:n}),n}throw new Error("cached value already set")}}}function it(e){return e==null}function st(e){let t=e.startsWith("^")?1:0,n=e.endsWith("$")?e.length-1:e.length;return e.slice(t,n)}function Qt(e,t){let n=e/t,r=Math.round(n),o=Number.EPSILON*Math.max(Math.abs(n),1);return Math.abs(n-r)<o?0:n-r}var mr=Symbol("evaluating");function L(e,t,n){let r;Object.defineProperty(e,t,{get(){if(r!==mr)return r===void 0&&(r=mr,r=n()),r},set(o){Object.defineProperty(e,t,{value:o})},configurable:!0})}function ja(e){return Object.create(Object.getPrototypeOf(e),Object.getOwnPropertyDescriptors(e))}function Se(e,t,n){Object.defineProperty(e,t,{value:n,writable:!0,enumerable:!0,configurable:!0})}function ke(...e){let t={};for(let n of e){let r=Object.getOwnPropertyDescriptors(n);Object.assign(t,r)}return Object.defineProperties({},t)}function Ua(e){return ke(e._zod.def)}function Ba(e,t){return t?t.reduce((n,r)=>n?.[r],e):e}function Wa(e){let t=Object.keys(e),n=t.map(r=>e[r]);return Promise.all(n).then(r=>{let o={};for(let i=0;i<t.length;i++)o[t[i]]=r[i];return o})}function Va(e=10){let t="abcdefghijklmnopqrstuvwxyz",n="";for(let r=0;r<e;r++)n+=t[Math.floor(Math.random()*t.length)];return n}function Tt(e){return JSON.stringify(e)}function en(e){return e.toLowerCase().trim().replace(/[^\w\s-]/g,"").replace(/[\s_-]+/g,"-").replace(/^-+|-+$/g,"")}var Pt="captureStackTrace"in Error?Error.captureStackTrace:(...e)=>{};function Be(e){return typeof e=="object"&&e!==null&&!Array.isArray(e)}var tn=ot(()=>{if(Ue.jitless||typeof navigator<"u"&&navigator?.userAgent?.includes("Cloudflare"))return!1;try{let e=Function;return new e(""),!0}catch{return!1}});function Ce(e){if(Be(e)===!1)return!1;let t=e.constructor;if(t===void 0||typeof t!="function")return!0;let n=t.prototype;return!(Be(n)===!1||Object.prototype.hasOwnProperty.call(n,"isPrototypeOf")===!1)}function gr(e){return Ce(e)?{...e}:Array.isArray(e)?[...e]:e instanceof Map?new Map(e):e instanceof Set?new Set(e):e}function Ga(e){let t=0;for(let n in e)Object.prototype.hasOwnProperty.call(e,n)&&t++;return t}var Ja=e=>{let t=typeof e;switch(t){case"undefined":return"undefined";case"string":return"string";case"number":return Number.isNaN(e)?"nan":"number";case"boolean":return"boolean";case"function":return"function";case"bigint":return"bigint";case"symbol":return"symbol";case"object":return Array.isArray(e)?"array":e===null?"null":e.then&&typeof e.then=="function"&&e.catch&&typeof e.catch=="function"?"promise":typeof Map<"u"&&e instanceof Map?"map":typeof Set<"u"&&e instanceof Set?"set":typeof Date<"u"&&e instanceof Date?"date":typeof File<"u"&&e instanceof File?"file":"object";default:throw new Error(`Unknown data type: ${t}`)}},nn=new Set(["string","number","symbol"]),_r=new Set(["string","number","bigint","boolean","symbol","undefined"]);function Ze(e){return e.replace(/[.*+?^${}()|[\]\\]/g,"\\$&")}function pe(e,t,n){let r=new e._zod.constr(t??e._zod.def);return(!t||n?.parent)&&(r._zod.parent=e),r}function z(e){let t=e;if(!t)return{};if(typeof t=="string")return{error:()=>t};if(t?.message!==void 0){if(t?.error!==void 0)throw new Error("Cannot specify both `message` and `error` params");t.error=t.message}return delete t.message,typeof t.error=="string"?{...t,error:()=>t.error}:t}function Ha(e){let t;return new Proxy({},{get(n,r,o){return t??(t=e()),Reflect.get(t,r,o)},set(n,r,o,i){return t??(t=e()),Reflect.set(t,r,o,i)},has(n,r){return t??(t=e()),Reflect.has(t,r)},deleteProperty(n,r){return t??(t=e()),Reflect.deleteProperty(t,r)},ownKeys(n){return t??(t=e()),Reflect.ownKeys(t)},getOwnPropertyDescriptor(n,r){return t??(t=e()),Reflect.getOwnPropertyDescriptor(t,r)},defineProperty(n,r,o){return t??(t=e()),Reflect.defineProperty(t,r,o)}})}function At(e){return typeof e=="bigint"?e.toString()+"n":typeof e=="string"?`"${e}"`:`${e}`}function rn(e){return Object.keys(e).filter(t=>e[t]._zod.optin==="optional"&&e[t]._zod.optout==="optional")}var on={safeint:[Number.MIN_SAFE_INTEGER,Number.MAX_SAFE_INTEGER],int32:[-2147483648,2147483647],uint32:[0,4294967295],float32:[-34028234663852886e22,34028234663852886e22],float64:[-Number.MAX_VALUE,Number.MAX_VALUE]},br={int64:[BigInt("-9223372036854775808"),BigInt("9223372036854775807")],uint64:[BigInt(0),BigInt("18446744073709551615")]};function qa(e,t){let n=e._zod.def,r=n.checks;if(r&&r.length>0)throw new Error(".pick() cannot be used on object schemas containing refinements");let i=ke(e._zod.def,{get shape(){let s={};for(let a in t){if(!(a in n.shape))throw new Error(`Unrecognized key: "${a}"`);t[a]&&(s[a]=n.shape[a])}return Se(this,"shape",s),s},checks:[]});return pe(e,i)}function Ya(e,t){let n=e._zod.def,r=n.checks;if(r&&r.length>0)throw new Error(".omit() cannot be used on object schemas containing refinements");let i=ke(e._zod.def,{get shape(){let s={...e._zod.def.shape};for(let a in t){if(!(a in n.shape))throw new Error(`Unrecognized key: "${a}"`);t[a]&&delete s[a]}return Se(this,"shape",s),s},checks:[]});return pe(e,i)}function Xa(e,t){if(!Ce(t))throw new Error("Invalid input to extend: expected a plain object");let n=e._zod.def.checks;if(n&&n.length>0){let i=e._zod.def.shape;for(let s in t)if(Object.getOwnPropertyDescriptor(i,s)!==void 0)throw new Error("Cannot overwrite keys on object schemas containing refinements. Use `.safeExtend()` instead.")}let o=ke(e._zod.def,{get shape(){let i={...e._zod.def.shape,...t};return Se(this,"shape",i),i}});return pe(e,o)}function Ka(e,t){if(!Ce(t))throw new Error("Invalid input to safeExtend: expected a plain object");let n=ke(e._zod.def,{get shape(){let r={...e._zod.def.shape,...t};return Se(this,"shape",r),r}});return pe(e,n)}function Qa(e,t){if(e._zod.def.checks?.length)throw new Error(".merge() cannot be used on object schemas containing refinements. Use .safeExtend() instead.");let n=ke(e._zod.def,{get shape(){let r={...e._zod.def.shape,...t._zod.def.shape};return Se(this,"shape",r),r},get catchall(){return t._zod.def.catchall},checks:t._zod.def.checks??[]});return pe(e,n)}function ec(e,t,n){let o=t._zod.def.checks;if(o&&o.length>0)throw new Error(".partial() cannot be used on object schemas containing refinements");let s=ke(t._zod.def,{get shape(){let a=t._zod.def.shape,u={...a};if(n)for(let l in n){if(!(l in a))throw new Error(`Unrecognized key: "${l}"`);n[l]&&(u[l]=e?new e({type:"optional",innerType:a[l]}):a[l])}else for(let l in a)u[l]=e?new e({type:"optional",innerType:a[l]}):a[l];return Se(this,"shape",u),u},checks:[]});return pe(t,s)}function tc(e,t,n){let r=ke(t._zod.def,{get shape(){let o=t._zod.def.shape,i={...o};if(n)for(let s in n){if(!(s in i))throw new Error(`Unrecognized key: "${s}"`);n[s]&&(i[s]=new e({type:"nonoptional",innerType:o[s]}))}else for(let s in o)i[s]=new e({type:"nonoptional",innerType:o[s]});return Se(this,"shape",i),i}});return pe(t,r)}function Te(e,t=0){if(e.aborted===!0)return!0;for(let n=t;n<e.issues.length;n++)if(e.issues[n]?.continue!==!0)return!0;return!1}function sn(e,t=0){if(e.aborted===!0)return!0;for(let n=t;n<e.issues.length;n++)if(e.issues[n]?.continue===!1)return!0;return!1}function at(e,t){return t.map(n=>{var r;return(r=n).path??(r.path=[]),n.path.unshift(e),n})}function nt(e){return typeof e=="string"?e:e?.message}function ye(e,t,n){let r=e.message?e.message:nt(e.inst?._zod.def?.error?.(e))??nt(t?.error?.(e))??nt(n.customError?.(e))??nt(n.localeError?.(e))??"Invalid input",{inst:o,continue:i,input:s,...a}=e;return a.path??(a.path=[]),a.message=r,t?.reportInput&&(a.input=s),a}function xr(e){return e instanceof Set?"set":e instanceof Map?"map":e instanceof File?"file":"unknown"}function ct(e){return Array.isArray(e)?"array":typeof e=="string"?"string":"unknown"}function an(e){let t=typeof e;switch(t){case"number":return Number.isNaN(e)?"nan":"number";case"object":{if(e===null)return"null";if(Array.isArray(e))return"array";let n=e;if(n&&Object.getPrototypeOf(n)!==Object.prototype&&"constructor"in n&&n.constructor)return n.constructor.name}}return t}function Ve(...e){let[t,n,r]=e;return typeof t=="string"?{message:t,code:"custom",input:n,inst:r}:{...t}}function nc(e){return Object.entries(e).filter(([t,n])=>Number.isNaN(Number.parseInt(t,10))).map(t=>t[1])}function yr(e){let t=atob(e),n=new Uint8Array(t.length);for(let r=0;r<t.length;r++)n[r]=t.charCodeAt(r);return n}function wr(e){let t="";for(let n=0;n<e.length;n++)t+=String.fromCharCode(e[n]);return btoa(t)}function rc(e){let t=e.replace(/-/g,"+").replace(/_/g,"/"),n="=".repeat((4-t.length%4)%4);return yr(t+n)}function oc(e){return wr(e).replace(/\+/g,"-").replace(/\//g,"_").replace(/=/g,"")}function ic(e){let t=e.replace(/^0x/,"");if(t.length%2!==0)throw new Error("Invalid hex string length");let n=new Uint8Array(t.length/2);for(let r=0;r<t.length;r+=2)n[r/2]=Number.parseInt(t.slice(r,r+2),16);return n}function sc(e){return Array.from(e).map(t=>t.toString(16).padStart(2,"0")).join("")}var Kt=class{constructor(...t){}};var vr=(e,t)=>{e.name="$ZodError",Object.defineProperty(e,"_zod",{value:e._zod,enumerable:!1}),Object.defineProperty(e,"issues",{value:t,enumerable:!1}),e.message=JSON.stringify(t,We,2),Object.defineProperty(e,"toString",{value:()=>e.message,enumerable:!1})},It=h("$ZodError",vr),cn=h("$ZodError",vr,{Parent:Error});function un(e,t=n=>n.message){let n={},r=[];for(let o of e.issues)o.path.length>0?(n[o.path[0]]=n[o.path[0]]||[],n[o.path[0]].push(t(o))):r.push(t(o));return{formErrors:r,fieldErrors:n}}function ln(e,t=n=>n.message){let n={_errors:[]},r=(o,i=[])=>{for(let s of o.issues)if(s.code==="invalid_union"&&s.errors.length)s.errors.map(a=>r({issues:a},[...i,...s.path]));else if(s.code==="invalid_key")r({issues:s.issues},[...i,...s.path]);else if(s.code==="invalid_element")r({issues:s.issues},[...i,...s.path]);else{let a=[...i,...s.path];if(a.length===0)n._errors.push(t(s));else{let u=n,l=0;for(;l<a.length;){let p=a[l];l===a.length-1?(u[p]=u[p]||{_errors:[]},u[p]._errors.push(t(s))):u[p]=u[p]||{_errors:[]},u=u[p],l++}}}};return r(e),n}function ac(e){let t=[],n=e.map(r=>typeof r=="object"?r.key:r);for(let r of n)typeof r=="number"?t.push(`[${r}]`):typeof r=="symbol"?t.push(`[${JSON.stringify(String(r))}]`):/[^\w$]/.test(r)?t.push(`[${JSON.stringify(r)}]`):(t.length&&t.push("."),t.push(r));return t.join("")}function ut(e){let t=[],n=[...e.issues].sort((r,o)=>(r.path??[]).length-(o.path??[]).length);for(let r of n)t.push(`\u2716 ${r.message}`),r.path?.length&&t.push(`  \u2192 at ${ac(r.path)}`);return t.join(`
`)}var Ot=e=>(t,n,r,o)=>{let i=r?{...r,async:!1}:{async:!1},s=t._zod.run({value:n,issues:[]},i);if(s instanceof Promise)throw new xe;if(s.issues.length){let a=new(o?.Err??e)(s.issues.map(u=>ye(u,i,le())));throw Pt(a,o?.callee),a}return s.value};var Nt=e=>async(t,n,r,o)=>{let i=r?{...r,async:!0}:{async:!0},s=t._zod.run({value:n,issues:[]},i);if(s instanceof Promise&&(s=await s),s.issues.length){let a=new(o?.Err??e)(s.issues.map(u=>ye(u,i,le())));throw Pt(a,o?.callee),a}return s.value};var lt=e=>(t,n,r)=>{let o=r?{...r,async:!1}:{async:!1},i=t._zod.run({value:n,issues:[]},o);if(i instanceof Promise)throw new xe;return i.issues.length?{success:!1,error:new(e??It)(i.issues.map(s=>ye(s,o,le())))}:{success:!0,data:i.value}},kr=lt(cn),pt=e=>async(t,n,r)=>{let o=r?{...r,async:!0}:{async:!0},i=t._zod.run({value:n,issues:[]},o);return i instanceof Promise&&(i=await i),i.issues.length?{success:!1,error:new e(i.issues.map(s=>ye(s,o,le())))}:{success:!0,data:i.value}},$r=pt(cn),zr=e=>(t,n,r)=>{let o=r?{...r,direction:"backward"}:{direction:"backward"};return Ot(e)(t,n,o)};var Er=e=>(t,n,r)=>Ot(e)(t,n,r);var Sr=e=>async(t,n,r)=>{let o=r?{...r,direction:"backward"}:{direction:"backward"};return Nt(e)(t,n,o)};var Tr=e=>async(t,n,r)=>Nt(e)(t,n,r);var Pr=e=>(t,n,r)=>{let o=r?{...r,direction:"backward"}:{direction:"backward"};return lt(e)(t,n,o)};var Ar=e=>(t,n,r)=>lt(e)(t,n,r);var Ir=e=>async(t,n,r)=>{let o=r?{...r,direction:"backward"}:{direction:"backward"};return pt(e)(t,n,o)};var Or=e=>async(t,n,r)=>pt(e)(t,n,r);var Nr=/^[cC][0-9a-z]{6,}$/,Cr=/^[0-9a-z]+$/,Zr=/^[0-9A-HJKMNP-TV-Za-hjkmnp-tv-z]{26}$/,Rr=/^[0-9a-vA-V]{20}$/,Lr=/^[A-Za-z0-9]{27}$/,Mr=/^[a-zA-Z0-9_-]{21}$/,Fr=/^P(?:(\d+W)|(?!.*W)(?=\d|T\d)(\d+Y)?(\d+M)?(\d+D)?(T(?=\d)(\d+H)?(\d+M)?(\d+([.,]\d+)?S)?)?)$/;var Dr=/^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$/,pn=e=>e?new RegExp(`^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-${e}[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12})$`):/^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$/;var jr=/^(?!\.)(?!.*\.\.)([A-Za-z0-9_'+\-\.]*)[A-Za-z0-9_+-]@([A-Za-z0-9][A-Za-z0-9\-]*\.)+[A-Za-z]{2,}$/;var uc="^(\\p{Extended_Pictographic}|\\p{Emoji_Component})+$";function Ur(){return new RegExp(uc,"u")}var Br=/^(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])$/,Wr=/^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:))$/;var Vr=/^((25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\/([0-9]|[1-2][0-9]|3[0-2])$/,Gr=/^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|::|([0-9a-fA-F]{1,4})?::([0-9a-fA-F]{1,4}:?){0,6})\/(12[0-8]|1[01][0-9]|[1-9]?[0-9])$/,Jr=/^$|^(?:[0-9a-zA-Z+/]{4})*(?:(?:[0-9a-zA-Z+/]{2}==)|(?:[0-9a-zA-Z+/]{3}=))?$/,dn=/^[A-Za-z0-9_-]*$/;var Hr=/^https?$/,qr=/^\+[1-9]\d{6,14}$/,Yr="(?:(?:\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\\d|30)|(?:02)-(?:0[1-9]|1\\d|2[0-8])))",Xr=new RegExp(`^${Yr}$`);function Kr(e){let t="(?:[01]\\d|2[0-3]):[0-5]\\d";return typeof e.precision=="number"?e.precision===-1?`${t}`:e.precision===0?`${t}:[0-5]\\d`:`${t}:[0-5]\\d\\.\\d{${e.precision}}`:`${t}(?::[0-5]\\d(?:\\.\\d+)?)?`}function Qr(e){return new RegExp(`^${Kr(e)}$`)}function eo(e){let t=Kr({precision:e.precision}),n=["Z"];e.local&&n.push(""),e.offset&&n.push("([+-](?:[01]\\d|2[0-3]):[0-5]\\d)");let r=`${t}(?:${n.join("|")})`;return new RegExp(`^${Yr}T(?:${r})$`)}var to=e=>{let t=e?`[\\s\\S]{${e?.minimum??0},${e?.maximum??""}}`:"[\\s\\S]*";return new RegExp(`^${t}$`)};var no=/^-?\d+$/,ro=/^-?\d+(?:\.\d+)?$/,oo=/^(?:true|false)$/i;var io=/^[^A-Z]*$/,so=/^[^a-z]*$/;var ee=h("$ZodCheck",(e,t)=>{var n;e._zod??(e._zod={}),e._zod.def=t,(n=e._zod).onattach??(n.onattach=[])}),ao={number:"number",bigint:"bigint",object:"date"},hn=h("$ZodCheckLessThan",(e,t)=>{ee.init(e,t);let n=ao[typeof t.value];e._zod.onattach.push(r=>{let o=r._zod.bag,i=(t.inclusive?o.maximum:o.exclusiveMaximum)??Number.POSITIVE_INFINITY;t.value<i&&(t.inclusive?o.maximum=t.value:o.exclusiveMaximum=t.value)}),e._zod.check=r=>{(t.inclusive?r.value<=t.value:r.value<t.value)||r.issues.push({origin:n,code:"too_big",maximum:typeof t.value=="object"?t.value.getTime():t.value,input:r.value,inclusive:t.inclusive,inst:e,continue:!t.abort})}}),fn=h("$ZodCheckGreaterThan",(e,t)=>{ee.init(e,t);let n=ao[typeof t.value];e._zod.onattach.push(r=>{let o=r._zod.bag,i=(t.inclusive?o.minimum:o.exclusiveMinimum)??Number.NEGATIVE_INFINITY;t.value>i&&(t.inclusive?o.minimum=t.value:o.exclusiveMinimum=t.value)}),e._zod.check=r=>{(t.inclusive?r.value>=t.value:r.value>t.value)||r.issues.push({origin:n,code:"too_small",minimum:typeof t.value=="object"?t.value.getTime():t.value,input:r.value,inclusive:t.inclusive,inst:e,continue:!t.abort})}}),co=h("$ZodCheckMultipleOf",(e,t)=>{ee.init(e,t),e._zod.onattach.push(n=>{var r;(r=n._zod.bag).multipleOf??(r.multipleOf=t.value)}),e._zod.check=n=>{if(typeof n.value!=typeof t.value)throw new Error("Cannot mix number and bigint in multiple_of check.");(typeof n.value=="bigint"?n.value%t.value===BigInt(0):Qt(n.value,t.value)===0)||n.issues.push({origin:typeof n.value,code:"not_multiple_of",divisor:t.value,input:n.value,inst:e,continue:!t.abort})}}),uo=h("$ZodCheckNumberFormat",(e,t)=>{ee.init(e,t),t.format=t.format||"float64";let n=t.format?.includes("int"),r=n?"int":"number",[o,i]=on[t.format];e._zod.onattach.push(s=>{let a=s._zod.bag;a.format=t.format,a.minimum=o,a.maximum=i,n&&(a.pattern=no)}),e._zod.check=s=>{let a=s.value;if(n){if(!Number.isInteger(a)){s.issues.push({expected:r,format:t.format,code:"invalid_type",continue:!1,input:a,inst:e});return}if(!Number.isSafeInteger(a)){a>0?s.issues.push({input:a,code:"too_big",maximum:Number.MAX_SAFE_INTEGER,note:"Integers must be within the safe integer range.",inst:e,origin:r,inclusive:!0,continue:!t.abort}):s.issues.push({input:a,code:"too_small",minimum:Number.MIN_SAFE_INTEGER,note:"Integers must be within the safe integer range.",inst:e,origin:r,inclusive:!0,continue:!t.abort});return}}a<o&&s.issues.push({origin:"number",input:a,code:"too_small",minimum:o,inclusive:!0,inst:e,continue:!t.abort}),a>i&&s.issues.push({origin:"number",input:a,code:"too_big",maximum:i,inclusive:!0,inst:e,continue:!t.abort})}});var lo=h("$ZodCheckMaxLength",(e,t)=>{var n;ee.init(e,t),(n=e._zod.def).when??(n.when=r=>{let o=r.value;return!it(o)&&o.length!==void 0}),e._zod.onattach.push(r=>{let o=r._zod.bag.maximum??Number.POSITIVE_INFINITY;t.maximum<o&&(r._zod.bag.maximum=t.maximum)}),e._zod.check=r=>{let o=r.value;if(o.length<=t.maximum)return;let s=ct(o);r.issues.push({origin:s,code:"too_big",maximum:t.maximum,inclusive:!0,input:o,inst:e,continue:!t.abort})}}),po=h("$ZodCheckMinLength",(e,t)=>{var n;ee.init(e,t),(n=e._zod.def).when??(n.when=r=>{let o=r.value;return!it(o)&&o.length!==void 0}),e._zod.onattach.push(r=>{let o=r._zod.bag.minimum??Number.NEGATIVE_INFINITY;t.minimum>o&&(r._zod.bag.minimum=t.minimum)}),e._zod.check=r=>{let o=r.value;if(o.length>=t.minimum)return;let s=ct(o);r.issues.push({origin:s,code:"too_small",minimum:t.minimum,inclusive:!0,input:o,inst:e,continue:!t.abort})}}),ho=h("$ZodCheckLengthEquals",(e,t)=>{var n;ee.init(e,t),(n=e._zod.def).when??(n.when=r=>{let o=r.value;return!it(o)&&o.length!==void 0}),e._zod.onattach.push(r=>{let o=r._zod.bag;o.minimum=t.length,o.maximum=t.length,o.length=t.length}),e._zod.check=r=>{let o=r.value,i=o.length;if(i===t.length)return;let s=ct(o),a=i>t.length;r.issues.push({origin:s,...a?{code:"too_big",maximum:t.length}:{code:"too_small",minimum:t.length},inclusive:!0,exact:!0,input:r.value,inst:e,continue:!t.abort})}}),ht=h("$ZodCheckStringFormat",(e,t)=>{var n,r;ee.init(e,t),e._zod.onattach.push(o=>{let i=o._zod.bag;i.format=t.format,t.pattern&&(i.patterns??(i.patterns=new Set),i.patterns.add(t.pattern))}),t.pattern?(n=e._zod).check??(n.check=o=>{t.pattern.lastIndex=0,!t.pattern.test(o.value)&&o.issues.push({origin:"string",code:"invalid_format",format:t.format,input:o.value,...t.pattern?{pattern:t.pattern.toString()}:{},inst:e,continue:!t.abort})}):(r=e._zod).check??(r.check=()=>{})}),fo=h("$ZodCheckRegex",(e,t)=>{ht.init(e,t),e._zod.check=n=>{t.pattern.lastIndex=0,!t.pattern.test(n.value)&&n.issues.push({origin:"string",code:"invalid_format",format:"regex",input:n.value,pattern:t.pattern.toString(),inst:e,continue:!t.abort})}}),mo=h("$ZodCheckLowerCase",(e,t)=>{t.pattern??(t.pattern=io),ht.init(e,t)}),go=h("$ZodCheckUpperCase",(e,t)=>{t.pattern??(t.pattern=so),ht.init(e,t)}),_o=h("$ZodCheckIncludes",(e,t)=>{ee.init(e,t);let n=Ze(t.includes),r=new RegExp(typeof t.position=="number"?`^.{${t.position}}${n}`:n);t.pattern=r,e._zod.onattach.push(o=>{let i=o._zod.bag;i.patterns??(i.patterns=new Set),i.patterns.add(r)}),e._zod.check=o=>{o.value.includes(t.includes,t.position)||o.issues.push({origin:"string",code:"invalid_format",format:"includes",includes:t.includes,input:o.value,inst:e,continue:!t.abort})}}),bo=h("$ZodCheckStartsWith",(e,t)=>{ee.init(e,t);let n=new RegExp(`^${Ze(t.prefix)}.*`);t.pattern??(t.pattern=n),e._zod.onattach.push(r=>{let o=r._zod.bag;o.patterns??(o.patterns=new Set),o.patterns.add(n)}),e._zod.check=r=>{r.value.startsWith(t.prefix)||r.issues.push({origin:"string",code:"invalid_format",format:"starts_with",prefix:t.prefix,input:r.value,inst:e,continue:!t.abort})}}),xo=h("$ZodCheckEndsWith",(e,t)=>{ee.init(e,t);let n=new RegExp(`.*${Ze(t.suffix)}$`);t.pattern??(t.pattern=n),e._zod.onattach.push(r=>{let o=r._zod.bag;o.patterns??(o.patterns=new Set),o.patterns.add(n)}),e._zod.check=r=>{r.value.endsWith(t.suffix)||r.issues.push({origin:"string",code:"invalid_format",format:"ends_with",suffix:t.suffix,input:r.value,inst:e,continue:!t.abort})}});var yo=h("$ZodCheckOverwrite",(e,t)=>{ee.init(e,t),e._zod.check=n=>{n.value=t.tx(n.value)}});var Ct=class{constructor(t=[]){this.content=[],this.indent=0,this&&(this.args=t)}indented(t){this.indent+=1,t(this),this.indent-=1}write(t){if(typeof t=="function"){t(this,{execution:"sync"}),t(this,{execution:"async"});return}let r=t.split(`
`).filter(s=>s),o=Math.min(...r.map(s=>s.length-s.trimStart().length)),i=r.map(s=>s.slice(o)).map(s=>" ".repeat(this.indent*2)+s);for(let s of i)this.content.push(s)}compile(){let t=Function,n=this?.args,o=[...(this?.content??[""]).map(i=>`  ${i}`)];return new t(...n,o.join(`
`))}};var vo={major:4,minor:4,patch:3};var V=h("$ZodType",(e,t)=>{var n;e??(e={}),e._zod.def=t,e._zod.bag=e._zod.bag||{},e._zod.version=vo;let r=[...e._zod.def.checks??[]];e._zod.traits.has("$ZodCheck")&&r.unshift(e);for(let o of r)for(let i of o._zod.onattach)i(e);if(r.length===0)(n=e._zod).deferred??(n.deferred=[]),e._zod.deferred?.push(()=>{e._zod.run=e._zod.parse});else{let o=(s,a,u)=>{let l=Te(s),p;for(let m of a){if(m._zod.def.when){if(sn(s)||!m._zod.def.when(s))continue}else if(l)continue;let d=s.issues.length,f=m._zod.check(s);if(f instanceof Promise&&u?.async===!1)throw new xe;if(p||f instanceof Promise)p=(p??Promise.resolve()).then(async()=>{await f,s.issues.length!==d&&(l||(l=Te(s,d)))});else{if(s.issues.length===d)continue;l||(l=Te(s,d))}}return p?p.then(()=>s):s},i=(s,a,u)=>{if(Te(s))return s.aborted=!0,s;let l=o(a,r,u);if(l instanceof Promise){if(u.async===!1)throw new xe;return l.then(p=>e._zod.parse(p,u))}return e._zod.parse(l,u)};e._zod.run=(s,a)=>{if(a.skipChecks)return e._zod.parse(s,a);if(a.direction==="backward"){let l=e._zod.parse({value:s.value,issues:[]},{...a,skipChecks:!0});return l instanceof Promise?l.then(p=>i(p,s,a)):i(l,s,a)}let u=e._zod.parse(s,a);if(u instanceof Promise){if(a.async===!1)throw new xe;return u.then(l=>o(l,r,a))}return o(u,r,a)}}L(e,"~standard",()=>({validate:o=>{try{let i=kr(e,o);return i.success?{value:i.data}:{issues:i.error?.issues}}catch{return $r(e,o).then(s=>s.success?{value:s.data}:{issues:s.error?.issues})}},vendor:"zod",version:1}))}),Lt=h("$ZodString",(e,t)=>{V.init(e,t),e._zod.pattern=[...e?._zod.bag?.patterns??[]].pop()??to(e._zod.bag),e._zod.parse=(n,r)=>{if(t.coerce)try{n.value=String(n.value)}catch{}return typeof n.value=="string"||n.issues.push({expected:"string",code:"invalid_type",input:n.value,inst:e}),n}}),j=h("$ZodStringFormat",(e,t)=>{ht.init(e,t),Lt.init(e,t)}),Io=h("$ZodGUID",(e,t)=>{t.pattern??(t.pattern=Dr),j.init(e,t)}),Oo=h("$ZodUUID",(e,t)=>{if(t.version){let r={v1:1,v2:2,v3:3,v4:4,v5:5,v6:6,v7:7,v8:8}[t.version];if(r===void 0)throw new Error(`Invalid UUID version: "${t.version}"`);t.pattern??(t.pattern=pn(r))}else t.pattern??(t.pattern=pn());j.init(e,t)}),No=h("$ZodEmail",(e,t)=>{t.pattern??(t.pattern=jr),j.init(e,t)}),Co=h("$ZodURL",(e,t)=>{j.init(e,t),e._zod.check=n=>{try{let r=n.value.trim();if(!t.normalize&&t.protocol?.source===Hr.source&&!/^https?:\/\//i.test(r)){n.issues.push({code:"invalid_format",format:"url",note:"Invalid URL format",input:n.value,inst:e,continue:!t.abort});return}let o=new URL(r);t.hostname&&(t.hostname.lastIndex=0,t.hostname.test(o.hostname)||n.issues.push({code:"invalid_format",format:"url",note:"Invalid hostname",pattern:t.hostname.source,input:n.value,inst:e,continue:!t.abort})),t.protocol&&(t.protocol.lastIndex=0,t.protocol.test(o.protocol.endsWith(":")?o.protocol.slice(0,-1):o.protocol)||n.issues.push({code:"invalid_format",format:"url",note:"Invalid protocol",pattern:t.protocol.source,input:n.value,inst:e,continue:!t.abort})),t.normalize?n.value=o.href:n.value=r;return}catch{n.issues.push({code:"invalid_format",format:"url",input:n.value,inst:e,continue:!t.abort})}}}),Zo=h("$ZodEmoji",(e,t)=>{t.pattern??(t.pattern=Ur()),j.init(e,t)}),Ro=h("$ZodNanoID",(e,t)=>{t.pattern??(t.pattern=Mr),j.init(e,t)}),Lo=h("$ZodCUID",(e,t)=>{t.pattern??(t.pattern=Nr),j.init(e,t)}),Mo=h("$ZodCUID2",(e,t)=>{t.pattern??(t.pattern=Cr),j.init(e,t)}),Fo=h("$ZodULID",(e,t)=>{t.pattern??(t.pattern=Zr),j.init(e,t)}),Do=h("$ZodXID",(e,t)=>{t.pattern??(t.pattern=Rr),j.init(e,t)}),jo=h("$ZodKSUID",(e,t)=>{t.pattern??(t.pattern=Lr),j.init(e,t)}),Uo=h("$ZodISODateTime",(e,t)=>{t.pattern??(t.pattern=eo(t)),j.init(e,t)}),Bo=h("$ZodISODate",(e,t)=>{t.pattern??(t.pattern=Xr),j.init(e,t)}),Wo=h("$ZodISOTime",(e,t)=>{t.pattern??(t.pattern=Qr(t)),j.init(e,t)}),Vo=h("$ZodISODuration",(e,t)=>{t.pattern??(t.pattern=Fr),j.init(e,t)}),Go=h("$ZodIPv4",(e,t)=>{t.pattern??(t.pattern=Br),j.init(e,t),e._zod.bag.format="ipv4"}),Jo=h("$ZodIPv6",(e,t)=>{t.pattern??(t.pattern=Wr),j.init(e,t),e._zod.bag.format="ipv6",e._zod.check=n=>{try{new URL(`http://[${n.value}]`)}catch{n.issues.push({code:"invalid_format",format:"ipv6",input:n.value,inst:e,continue:!t.abort})}}});var Ho=h("$ZodCIDRv4",(e,t)=>{t.pattern??(t.pattern=Vr),j.init(e,t)}),qo=h("$ZodCIDRv6",(e,t)=>{t.pattern??(t.pattern=Gr),j.init(e,t),e._zod.check=n=>{let r=n.value.split("/");try{if(r.length!==2)throw new Error;let[o,i]=r;if(!i)throw new Error;let s=Number(i);if(`${s}`!==i)throw new Error;if(s<0||s>128)throw new Error;new URL(`http://[${o}]`)}catch{n.issues.push({code:"invalid_format",format:"cidrv6",input:n.value,inst:e,continue:!t.abort})}}});function Yo(e){if(e==="")return!0;if(/\s/.test(e)||e.length%4!==0)return!1;try{return atob(e),!0}catch{return!1}}var Xo=h("$ZodBase64",(e,t)=>{t.pattern??(t.pattern=Jr),j.init(e,t),e._zod.bag.contentEncoding="base64",e._zod.check=n=>{Yo(n.value)||n.issues.push({code:"invalid_format",format:"base64",input:n.value,inst:e,continue:!t.abort})}});function lc(e){if(!dn.test(e))return!1;let t=e.replace(/[-_]/g,r=>r==="-"?"+":"/"),n=t.padEnd(Math.ceil(t.length/4)*4,"=");return Yo(n)}var Ko=h("$ZodBase64URL",(e,t)=>{t.pattern??(t.pattern=dn),j.init(e,t),e._zod.bag.contentEncoding="base64url",e._zod.check=n=>{lc(n.value)||n.issues.push({code:"invalid_format",format:"base64url",input:n.value,inst:e,continue:!t.abort})}}),Qo=h("$ZodE164",(e,t)=>{t.pattern??(t.pattern=qr),j.init(e,t)});function pc(e,t=null){try{let n=e.split(".");if(n.length!==3)return!1;let[r]=n;if(!r)return!1;let o=JSON.parse(atob(r));return!("typ"in o&&o?.typ!=="JWT"||!o.alg||t&&(!("alg"in o)||o.alg!==t))}catch{return!1}}var ei=h("$ZodJWT",(e,t)=>{j.init(e,t),e._zod.check=n=>{pc(n.value,t.alg)||n.issues.push({code:"invalid_format",format:"jwt",input:n.value,inst:e,continue:!t.abort})}});var gn=h("$ZodNumber",(e,t)=>{V.init(e,t),e._zod.pattern=e._zod.bag.pattern??ro,e._zod.parse=(n,r)=>{if(t.coerce)try{n.value=Number(n.value)}catch{}let o=n.value;if(typeof o=="number"&&!Number.isNaN(o)&&Number.isFinite(o))return n;let i=typeof o=="number"?Number.isNaN(o)?"NaN":Number.isFinite(o)?void 0:"Infinity":void 0;return n.issues.push({expected:"number",code:"invalid_type",input:o,inst:e,...i?{received:i}:{}}),n}}),ti=h("$ZodNumberFormat",(e,t)=>{uo.init(e,t),gn.init(e,t)}),ni=h("$ZodBoolean",(e,t)=>{V.init(e,t),e._zod.pattern=oo,e._zod.parse=(n,r)=>{if(t.coerce)try{n.value=!!n.value}catch{}let o=n.value;return typeof o=="boolean"||n.issues.push({expected:"boolean",code:"invalid_type",input:o,inst:e}),n}});var ri=h("$ZodUnknown",(e,t)=>{V.init(e,t),e._zod.parse=n=>n}),oi=h("$ZodNever",(e,t)=>{V.init(e,t),e._zod.parse=(n,r)=>(n.issues.push({expected:"never",code:"invalid_type",input:n.value,inst:e}),n)});function ko(e,t,n){e.issues.length&&t.issues.push(...at(n,e.issues)),t.value[n]=e.value}var ii=h("$ZodArray",(e,t)=>{V.init(e,t),e._zod.parse=(n,r)=>{let o=n.value;if(!Array.isArray(o))return n.issues.push({expected:"array",code:"invalid_type",input:o,inst:e}),n;n.value=Array(o.length);let i=[];for(let s=0;s<o.length;s++){let a=o[s],u=t.element._zod.run({value:a,issues:[]},r);u instanceof Promise?i.push(u.then(l=>ko(l,n,s))):ko(u,n,s)}return i.length?Promise.all(i).then(()=>n):n}});function Rt(e,t,n,r,o,i){let s=n in r;if(e.issues.length){if(o&&i&&!s)return;t.issues.push(...at(n,e.issues))}if(!s&&!o){e.issues.length||t.issues.push({code:"invalid_type",expected:"nonoptional",input:void 0,path:[n]});return}e.value===void 0?s&&(t.value[n]=void 0):t.value[n]=e.value}function si(e){let t=Object.keys(e.shape);for(let r of t)if(!e.shape?.[r]?._zod?.traits?.has("$ZodType"))throw new Error(`Invalid element at key "${r}": expected a Zod schema`);let n=rn(e.shape);return{...e,keys:t,keySet:new Set(t),numKeys:t.length,optionalKeys:new Set(n)}}function ai(e,t,n,r,o,i){let s=[],a=o.keySet,u=o.catchall._zod,l=u.def.type,p=u.optin==="optional",m=u.optout==="optional";for(let d in t){if(d==="__proto__"||a.has(d))continue;if(l==="never"){s.push(d);continue}let f=u.run({value:t[d],issues:[]},r);f instanceof Promise?e.push(f.then(g=>Rt(g,n,d,t,p,m))):Rt(f,n,d,t,p,m)}return s.length&&n.issues.push({code:"unrecognized_keys",keys:s,input:t,inst:i}),e.length?Promise.all(e).then(()=>n):n}var dc=h("$ZodObject",(e,t)=>{if(V.init(e,t),!Object.getOwnPropertyDescriptor(t,"shape")?.get){let a=t.shape;Object.defineProperty(t,"shape",{get:()=>{let u={...a};return Object.defineProperty(t,"shape",{value:u}),u}})}let r=ot(()=>si(t));L(e._zod,"propValues",()=>{let a=t.shape,u={};for(let l in a){let p=a[l]._zod;if(p.values){u[l]??(u[l]=new Set);for(let m of p.values)u[l].add(m)}}return u});let o=Be,i=t.catchall,s;e._zod.parse=(a,u)=>{s??(s=r.value);let l=a.value;if(!o(l))return a.issues.push({expected:"object",code:"invalid_type",input:l,inst:e}),a;a.value={};let p=[],m=s.shape;for(let d of s.keys){let f=m[d],g=f._zod.optin==="optional",O=f._zod.optout==="optional",w=f._zod.run({value:l[d],issues:[]},u);w instanceof Promise?p.push(w.then(C=>Rt(C,a,d,l,g,O))):Rt(w,a,d,l,g,O)}return i?ai(p,l,a,u,r.value,e):p.length?Promise.all(p).then(()=>a):a}}),ci=h("$ZodObjectJIT",(e,t)=>{dc.init(e,t);let n=e._zod.parse,r=ot(()=>si(t)),o=d=>{let f=new Ct(["shape","payload","ctx"]),g=r.value,O=A=>{let b=Tt(A);return`shape[${b}]._zod.run({ value: input[${b}], issues: [] }, ctx)`};f.write("const input = payload.value;");let w=Object.create(null),C=0;for(let A of g.keys)w[A]=`key_${C++}`;f.write("const newResult = {};");for(let A of g.keys){let b=w[A],k=Tt(A),W=d[A],R=W?._zod?.optin==="optional",fe=W?._zod?.optout==="optional";f.write(`const ${b} = ${O(A)};`),R&&fe?f.write(`
        if (${b}.issues.length) {
          if (${k} in input) {
            payload.issues = payload.issues.concat(${b}.issues.map(iss => ({
              ...iss,
              path: iss.path ? [${k}, ...iss.path] : [${k}]
            })));
          }
        }
        
        if (${b}.value === undefined) {
          if (${k} in input) {
            newResult[${k}] = undefined;
          }
        } else {
          newResult[${k}] = ${b}.value;
        }
        
      `):R?f.write(`
        if (${b}.issues.length) {
          payload.issues = payload.issues.concat(${b}.issues.map(iss => ({
            ...iss,
            path: iss.path ? [${k}, ...iss.path] : [${k}]
          })));
        }
        
        if (${b}.value === undefined) {
          if (${k} in input) {
            newResult[${k}] = undefined;
          }
        } else {
          newResult[${k}] = ${b}.value;
        }
        
      `):f.write(`
        const ${b}_present = ${k} in input;
        if (${b}.issues.length) {
          payload.issues = payload.issues.concat(${b}.issues.map(iss => ({
            ...iss,
            path: iss.path ? [${k}, ...iss.path] : [${k}]
          })));
        }
        if (!${b}_present && !${b}.issues.length) {
          payload.issues.push({
            code: "invalid_type",
            expected: "nonoptional",
            input: undefined,
            path: [${k}]
          });
        }

        if (${b}_present) {
          if (${b}.value === undefined) {
            newResult[${k}] = undefined;
          } else {
            newResult[${k}] = ${b}.value;
          }
        }

      `)}f.write("payload.value = newResult;"),f.write("return payload;");let P=f.compile();return(A,b)=>P(d,A,b)},i,s=Be,a=!Ue.jitless,l=a&&tn.value,p=t.catchall,m;e._zod.parse=(d,f)=>{m??(m=r.value);let g=d.value;return s(g)?a&&l&&f?.async===!1&&f.jitless!==!0?(i||(i=o(t.shape)),d=i(d,f),p?ai([],g,d,f,m,e):d):n(d,f):(d.issues.push({expected:"object",code:"invalid_type",input:g,inst:e}),d)}});function $o(e,t,n,r){for(let i of e)if(i.issues.length===0)return t.value=i.value,t;let o=e.filter(i=>!Te(i));return o.length===1?(t.value=o[0].value,o[0]):(t.issues.push({code:"invalid_union",input:t.value,inst:n,errors:e.map(i=>i.issues.map(s=>ye(s,r,le())))}),t)}var ui=h("$ZodUnion",(e,t)=>{V.init(e,t),L(e._zod,"optin",()=>t.options.some(r=>r._zod.optin==="optional")?"optional":void 0),L(e._zod,"optout",()=>t.options.some(r=>r._zod.optout==="optional")?"optional":void 0),L(e._zod,"values",()=>{if(t.options.every(r=>r._zod.values))return new Set(t.options.flatMap(r=>Array.from(r._zod.values)))}),L(e._zod,"pattern",()=>{if(t.options.every(r=>r._zod.pattern)){let r=t.options.map(o=>o._zod.pattern);return new RegExp(`^(${r.map(o=>st(o.source)).join("|")})$`)}});let n=t.options.length===1?t.options[0]._zod.run:null;e._zod.parse=(r,o)=>{if(n)return n(r,o);let i=!1,s=[];for(let a of t.options){let u=a._zod.run({value:r.value,issues:[]},o);if(u instanceof Promise)s.push(u),i=!0;else{if(u.issues.length===0)return u;s.push(u)}}return i?Promise.all(s).then(a=>$o(a,r,e,o)):$o(s,r,e,o)}});var li=h("$ZodIntersection",(e,t)=>{V.init(e,t),e._zod.parse=(n,r)=>{let o=n.value,i=t.left._zod.run({value:o,issues:[]},r),s=t.right._zod.run({value:o,issues:[]},r);return i instanceof Promise||s instanceof Promise?Promise.all([i,s]).then(([u,l])=>zo(n,u,l)):zo(n,i,s)}});function mn(e,t){if(e===t)return{valid:!0,data:e};if(e instanceof Date&&t instanceof Date&&+e==+t)return{valid:!0,data:e};if(Ce(e)&&Ce(t)){let n=Object.keys(t),r=Object.keys(e).filter(i=>n.indexOf(i)!==-1),o={...e,...t};for(let i of r){let s=mn(e[i],t[i]);if(!s.valid)return{valid:!1,mergeErrorPath:[i,...s.mergeErrorPath]};o[i]=s.data}return{valid:!0,data:o}}if(Array.isArray(e)&&Array.isArray(t)){if(e.length!==t.length)return{valid:!1,mergeErrorPath:[]};let n=[];for(let r=0;r<e.length;r++){let o=e[r],i=t[r],s=mn(o,i);if(!s.valid)return{valid:!1,mergeErrorPath:[r,...s.mergeErrorPath]};n.push(s.data)}return{valid:!0,data:n}}return{valid:!1,mergeErrorPath:[]}}function zo(e,t,n){let r=new Map,o;for(let a of t.issues)if(a.code==="unrecognized_keys"){o??(o=a);for(let u of a.keys)r.has(u)||r.set(u,{}),r.get(u).l=!0}else e.issues.push(a);for(let a of n.issues)if(a.code==="unrecognized_keys")for(let u of a.keys)r.has(u)||r.set(u,{}),r.get(u).r=!0;else e.issues.push(a);let i=[...r].filter(([,a])=>a.l&&a.r).map(([a])=>a);if(i.length&&o&&e.issues.push({...o,keys:i}),Te(e))return e;let s=mn(t.value,n.value);if(!s.valid)throw new Error(`Unmergable intersection. Error path: ${JSON.stringify(s.mergeErrorPath)}`);return e.value=s.data,e}var pi=h("$ZodEnum",(e,t)=>{V.init(e,t);let n=rt(t.entries),r=new Set(n);e._zod.values=r,e._zod.pattern=new RegExp(`^(${n.filter(o=>nn.has(typeof o)).map(o=>typeof o=="string"?Ze(o):o.toString()).join("|")})$`),e._zod.parse=(o,i)=>{let s=o.value;return r.has(s)||o.issues.push({code:"invalid_value",values:n,input:s,inst:e}),o}});var di=h("$ZodTransform",(e,t)=>{V.init(e,t),e._zod.optin="optional",e._zod.parse=(n,r)=>{if(r.direction==="backward")throw new je(e.constructor.name);let o=t.transform(n.value,n);if(r.async)return(o instanceof Promise?o:Promise.resolve(o)).then(s=>(n.value=s,n.fallback=!0,n));if(o instanceof Promise)throw new xe;return n.value=o,n.fallback=!0,n}});function Eo(e,t){return t===void 0&&(e.issues.length||e.fallback)?{issues:[],value:void 0}:e}var _n=h("$ZodOptional",(e,t)=>{V.init(e,t),e._zod.optin="optional",e._zod.optout="optional",L(e._zod,"values",()=>t.innerType._zod.values?new Set([...t.innerType._zod.values,void 0]):void 0),L(e._zod,"pattern",()=>{let n=t.innerType._zod.pattern;return n?new RegExp(`^(${st(n.source)})?$`):void 0}),e._zod.parse=(n,r)=>{if(t.innerType._zod.optin==="optional"){let o=n.value,i=t.innerType._zod.run(n,r);return i instanceof Promise?i.then(s=>Eo(s,o)):Eo(i,o)}return n.value===void 0?n:t.innerType._zod.run(n,r)}}),hi=h("$ZodExactOptional",(e,t)=>{_n.init(e,t),L(e._zod,"values",()=>t.innerType._zod.values),L(e._zod,"pattern",()=>t.innerType._zod.pattern),e._zod.parse=(n,r)=>t.innerType._zod.run(n,r)}),fi=h("$ZodNullable",(e,t)=>{V.init(e,t),L(e._zod,"optin",()=>t.innerType._zod.optin),L(e._zod,"optout",()=>t.innerType._zod.optout),L(e._zod,"pattern",()=>{let n=t.innerType._zod.pattern;return n?new RegExp(`^(${st(n.source)}|null)$`):void 0}),L(e._zod,"values",()=>t.innerType._zod.values?new Set([...t.innerType._zod.values,null]):void 0),e._zod.parse=(n,r)=>n.value===null?n:t.innerType._zod.run(n,r)}),mi=h("$ZodDefault",(e,t)=>{V.init(e,t),e._zod.optin="optional",L(e._zod,"values",()=>t.innerType._zod.values),e._zod.parse=(n,r)=>{if(r.direction==="backward")return t.innerType._zod.run(n,r);if(n.value===void 0)return n.value=t.defaultValue,n;let o=t.innerType._zod.run(n,r);return o instanceof Promise?o.then(i=>So(i,t)):So(o,t)}});function So(e,t){return e.value===void 0&&(e.value=t.defaultValue),e}var gi=h("$ZodPrefault",(e,t)=>{V.init(e,t),e._zod.optin="optional",L(e._zod,"values",()=>t.innerType._zod.values),e._zod.parse=(n,r)=>(r.direction==="backward"||n.value===void 0&&(n.value=t.defaultValue),t.innerType._zod.run(n,r))}),_i=h("$ZodNonOptional",(e,t)=>{V.init(e,t),L(e._zod,"values",()=>{let n=t.innerType._zod.values;return n?new Set([...n].filter(r=>r!==void 0)):void 0}),e._zod.parse=(n,r)=>{let o=t.innerType._zod.run(n,r);return o instanceof Promise?o.then(i=>To(i,e)):To(o,e)}});function To(e,t){return!e.issues.length&&e.value===void 0&&e.issues.push({code:"invalid_type",expected:"nonoptional",input:e.value,inst:t}),e}var bi=h("$ZodCatch",(e,t)=>{V.init(e,t),e._zod.optin="optional",L(e._zod,"optout",()=>t.innerType._zod.optout),L(e._zod,"values",()=>t.innerType._zod.values),e._zod.parse=(n,r)=>{if(r.direction==="backward")return t.innerType._zod.run(n,r);let o=t.innerType._zod.run(n,r);return o instanceof Promise?o.then(i=>(n.value=i.value,i.issues.length&&(n.value=t.catchValue({...n,error:{issues:i.issues.map(s=>ye(s,r,le()))},input:n.value}),n.issues=[],n.fallback=!0),n)):(n.value=o.value,o.issues.length&&(n.value=t.catchValue({...n,error:{issues:o.issues.map(i=>ye(i,r,le()))},input:n.value}),n.issues=[],n.fallback=!0),n)}});var xi=h("$ZodPipe",(e,t)=>{V.init(e,t),L(e._zod,"values",()=>t.in._zod.values),L(e._zod,"optin",()=>t.in._zod.optin),L(e._zod,"optout",()=>t.out._zod.optout),L(e._zod,"propValues",()=>t.in._zod.propValues),e._zod.parse=(n,r)=>{if(r.direction==="backward"){let i=t.out._zod.run(n,r);return i instanceof Promise?i.then(s=>Zt(s,t.in,r)):Zt(i,t.in,r)}let o=t.in._zod.run(n,r);return o instanceof Promise?o.then(i=>Zt(i,t.out,r)):Zt(o,t.out,r)}});function Zt(e,t,n){return e.issues.length?(e.aborted=!0,e):t._zod.run({value:e.value,issues:e.issues,fallback:e.fallback},n)}var yi=h("$ZodReadonly",(e,t)=>{V.init(e,t),L(e._zod,"propValues",()=>t.innerType._zod.propValues),L(e._zod,"values",()=>t.innerType._zod.values),L(e._zod,"optin",()=>t.innerType?._zod?.optin),L(e._zod,"optout",()=>t.innerType?._zod?.optout),e._zod.parse=(n,r)=>{if(r.direction==="backward")return t.innerType._zod.run(n,r);let o=t.innerType._zod.run(n,r);return o instanceof Promise?o.then(Po):Po(o)}});function Po(e){return e.value=Object.freeze(e.value),e}var wi=h("$ZodCustom",(e,t)=>{ee.init(e,t),V.init(e,t),e._zod.parse=(n,r)=>n,e._zod.check=n=>{let r=n.value,o=t.fn(r);if(o instanceof Promise)return o.then(i=>Ao(i,n,r,e));Ao(o,n,r,e)}});function Ao(e,t,n,r){if(!e){let o={code:"custom",input:n,inst:r,path:[...r._zod.def.path??[]],continue:!r._zod.def.abort};r._zod.def.params&&(o.params=r._zod.def.params),t.issues.push(Ve(o))}}var hc=()=>{let e={string:{unit:"characters",verb:"to have"},file:{unit:"bytes",verb:"to have"},array:{unit:"items",verb:"to have"},set:{unit:"items",verb:"to have"},map:{unit:"entries",verb:"to have"}};function t(o){return e[o]??null}let n={regex:"input",email:"email address",url:"URL",emoji:"emoji",uuid:"UUID",uuidv4:"UUIDv4",uuidv6:"UUIDv6",nanoid:"nanoid",guid:"GUID",cuid:"cuid",cuid2:"cuid2",ulid:"ULID",xid:"XID",ksuid:"KSUID",datetime:"ISO datetime",date:"ISO date",time:"ISO time",duration:"ISO duration",ipv4:"IPv4 address",ipv6:"IPv6 address",mac:"MAC address",cidrv4:"IPv4 range",cidrv6:"IPv6 range",base64:"base64-encoded string",base64url:"base64url-encoded string",json_string:"JSON string",e164:"E.164 number",jwt:"JWT",template_literal:"input"},r={nan:"NaN"};return o=>{switch(o.code){case"invalid_type":{let i=r[o.expected]??o.expected,s=an(o.input),a=r[s]??s;return`Invalid input: expected ${i}, received ${a}`}case"invalid_value":return o.values.length===1?`Invalid input: expected ${At(o.values[0])}`:`Invalid option: expected one of ${St(o.values,"|")}`;case"too_big":{let i=o.inclusive?"<=":"<",s=t(o.origin);return s?`Too big: expected ${o.origin??"value"} to have ${i}${o.maximum.toString()} ${s.unit??"elements"}`:`Too big: expected ${o.origin??"value"} to be ${i}${o.maximum.toString()}`}case"too_small":{let i=o.inclusive?">=":">",s=t(o.origin);return s?`Too small: expected ${o.origin} to have ${i}${o.minimum.toString()} ${s.unit}`:`Too small: expected ${o.origin} to be ${i}${o.minimum.toString()}`}case"invalid_format":{let i=o;return i.format==="starts_with"?`Invalid string: must start with "${i.prefix}"`:i.format==="ends_with"?`Invalid string: must end with "${i.suffix}"`:i.format==="includes"?`Invalid string: must include "${i.includes}"`:i.format==="regex"?`Invalid string: must match pattern ${i.pattern}`:`Invalid ${n[i.format]??o.format}`}case"not_multiple_of":return`Invalid number: must be a multiple of ${o.divisor}`;case"unrecognized_keys":return`Unrecognized key${o.keys.length>1?"s":""}: ${St(o.keys,", ")}`;case"invalid_key":return`Invalid key in ${o.origin}`;case"invalid_union":return o.options&&Array.isArray(o.options)&&o.options.length>0?`Invalid discriminator value. Expected ${o.options.map(s=>`'${s}'`).join(" | ")}`:"Invalid input";case"invalid_element":return`Invalid value in ${o.origin}`;default:return"Invalid input"}}};function vi(){return{localeError:hc()}}var ki;var bn=class{constructor(){this._map=new WeakMap,this._idmap=new Map}add(t,...n){let r=n[0];return this._map.set(t,r),r&&typeof r=="object"&&"id"in r&&this._idmap.set(r.id,t),this}clear(){return this._map=new WeakMap,this._idmap=new Map,this}remove(t){let n=this._map.get(t);return n&&typeof n=="object"&&"id"in n&&this._idmap.delete(n.id),this._map.delete(t),this}get(t){let n=t._zod.parent;if(n){let r={...this.get(n)??{}};delete r.id;let o={...r,...this._map.get(t)};return Object.keys(o).length?o:void 0}return this._map.get(t)}has(t){return this._map.has(t)}};function $i(){return new bn}(ki=globalThis).__zod_globalRegistry??(ki.__zod_globalRegistry=$i());var Pe=globalThis.__zod_globalRegistry;function zi(e,t){return new e({type:"string",...z(t)})}function Ei(e,t){return new e({type:"string",format:"email",check:"string_format",abort:!1,...z(t)})}function xn(e,t){return new e({type:"string",format:"guid",check:"string_format",abort:!1,...z(t)})}function Si(e,t){return new e({type:"string",format:"uuid",check:"string_format",abort:!1,...z(t)})}function Ti(e,t){return new e({type:"string",format:"uuid",check:"string_format",abort:!1,version:"v4",...z(t)})}function Pi(e,t){return new e({type:"string",format:"uuid",check:"string_format",abort:!1,version:"v6",...z(t)})}function Ai(e,t){return new e({type:"string",format:"uuid",check:"string_format",abort:!1,version:"v7",...z(t)})}function Ii(e,t){return new e({type:"string",format:"url",check:"string_format",abort:!1,...z(t)})}function Oi(e,t){return new e({type:"string",format:"emoji",check:"string_format",abort:!1,...z(t)})}function Ni(e,t){return new e({type:"string",format:"nanoid",check:"string_format",abort:!1,...z(t)})}function Ci(e,t){return new e({type:"string",format:"cuid",check:"string_format",abort:!1,...z(t)})}function Zi(e,t){return new e({type:"string",format:"cuid2",check:"string_format",abort:!1,...z(t)})}function Ri(e,t){return new e({type:"string",format:"ulid",check:"string_format",abort:!1,...z(t)})}function Li(e,t){return new e({type:"string",format:"xid",check:"string_format",abort:!1,...z(t)})}function Mi(e,t){return new e({type:"string",format:"ksuid",check:"string_format",abort:!1,...z(t)})}function Fi(e,t){return new e({type:"string",format:"ipv4",check:"string_format",abort:!1,...z(t)})}function Di(e,t){return new e({type:"string",format:"ipv6",check:"string_format",abort:!1,...z(t)})}function ji(e,t){return new e({type:"string",format:"cidrv4",check:"string_format",abort:!1,...z(t)})}function Ui(e,t){return new e({type:"string",format:"cidrv6",check:"string_format",abort:!1,...z(t)})}function Bi(e,t){return new e({type:"string",format:"base64",check:"string_format",abort:!1,...z(t)})}function Wi(e,t){return new e({type:"string",format:"base64url",check:"string_format",abort:!1,...z(t)})}function Vi(e,t){return new e({type:"string",format:"e164",check:"string_format",abort:!1,...z(t)})}function Gi(e,t){return new e({type:"string",format:"jwt",check:"string_format",abort:!1,...z(t)})}function Ji(e,t){return new e({type:"string",format:"datetime",check:"string_format",offset:!1,local:!1,precision:null,...z(t)})}function Hi(e,t){return new e({type:"string",format:"date",check:"string_format",...z(t)})}function qi(e,t){return new e({type:"string",format:"time",check:"string_format",precision:null,...z(t)})}function Yi(e,t){return new e({type:"string",format:"duration",check:"string_format",...z(t)})}function Xi(e,t){return new e({type:"number",checks:[],...z(t)})}function Ki(e,t){return new e({type:"number",check:"number_format",abort:!1,format:"safeint",...z(t)})}function Qi(e,t){return new e({type:"boolean",...z(t)})}function es(e){return new e({type:"unknown"})}function ts(e,t){return new e({type:"never",...z(t)})}function Mt(e,t){return new hn({check:"less_than",...z(t),value:e,inclusive:!1})}function ft(e,t){return new hn({check:"less_than",...z(t),value:e,inclusive:!0})}function Ft(e,t){return new fn({check:"greater_than",...z(t),value:e,inclusive:!1})}function mt(e,t){return new fn({check:"greater_than",...z(t),value:e,inclusive:!0})}function Dt(e,t){return new co({check:"multiple_of",...z(t),value:e})}function jt(e,t){return new lo({check:"max_length",...z(t),maximum:e})}function Ge(e,t){return new po({check:"min_length",...z(t),minimum:e})}function Ut(e,t){return new ho({check:"length_equals",...z(t),length:e})}function yn(e,t){return new fo({check:"string_format",format:"regex",...z(t),pattern:e})}function wn(e){return new mo({check:"string_format",format:"lowercase",...z(e)})}function vn(e){return new go({check:"string_format",format:"uppercase",...z(e)})}function kn(e,t){return new _o({check:"string_format",format:"includes",...z(t),includes:e})}function $n(e,t){return new bo({check:"string_format",format:"starts_with",...z(t),prefix:e})}function zn(e,t){return new xo({check:"string_format",format:"ends_with",...z(t),suffix:e})}function Ae(e){return new yo({check:"overwrite",tx:e})}function En(e){return Ae(t=>t.normalize(e))}function Sn(){return Ae(e=>e.trim())}function Tn(){return Ae(e=>e.toLowerCase())}function Pn(){return Ae(e=>e.toUpperCase())}function An(){return Ae(e=>en(e))}function ns(e,t,n){return new e({type:"array",element:t,...z(n)})}function rs(e,t,n){return new e({type:"custom",check:"custom",fn:t,...z(n)})}function os(e,t){let n=fc(r=>(r.addIssue=o=>{if(typeof o=="string")r.issues.push(Ve(o,r.value,n._zod.def));else{let i=o;i.fatal&&(i.continue=!1),i.code??(i.code="custom"),i.input??(i.input=r.value),i.inst??(i.inst=n),i.continue??(i.continue=!n._zod.def.abort),r.issues.push(Ve(i))}},e(r.value,r)),t);return n}function fc(e,t){let n=new ee({check:"custom",...z(t)});return n._zod.check=e,n}function _t(e){let t=e?.target??"draft-2020-12";return t==="draft-4"&&(t="draft-04"),t==="draft-7"&&(t="draft-07"),{processors:e.processors??{},metadataRegistry:e?.metadata??Pe,target:t,unrepresentable:e?.unrepresentable??"throw",override:e?.override??(()=>{}),io:e?.io??"output",counter:0,seen:new Map,cycles:e?.cycles??"ref",reused:e?.reused??"inline",external:e?.external??void 0}}function F(e,t,n={path:[],schemaPath:[]}){var r;let o=e._zod.def,i=t.seen.get(e);if(i)return i.count++,n.schemaPath.includes(e)&&(i.cycle=n.path),i.schema;let s={schema:{},count:1,cycle:void 0,path:n.path};t.seen.set(e,s);let a=e._zod.toJSONSchema?.();if(a)s.schema=a;else{let p={...n,schemaPath:[...n.schemaPath,e],path:n.path};if(e._zod.processJSONSchema)e._zod.processJSONSchema(t,s.schema,p);else{let d=s.schema,f=t.processors[o.type];if(!f)throw new Error(`[toJSONSchema]: Non-representable type encountered: ${o.type}`);f(e,t,d,p)}let m=e._zod.parent;m&&(s.ref||(s.ref=m),F(m,t,p),t.seen.get(m).isParent=!0)}let u=t.metadataRegistry.get(e);return u&&Object.assign(s.schema,u),t.io==="input"&&ie(e)&&(delete s.schema.examples,delete s.schema.default),t.io==="input"&&"_prefault"in s.schema&&((r=s.schema).default??(r.default=s.schema._prefault)),delete s.schema._prefault,t.seen.get(e).schema}function bt(e,t){let n=e.seen.get(t);if(!n)throw new Error("Unprocessed schema. This is a bug in Zod.");let r=new Map;for(let s of e.seen.entries()){let a=e.metadataRegistry.get(s[0])?.id;if(a){let u=r.get(a);if(u&&u!==s[0])throw new Error(`Duplicate schema id "${a}" detected during JSON Schema conversion. Two different schemas cannot share the same id when converted together.`);r.set(a,s[0])}}let o=s=>{let a=e.target==="draft-2020-12"?"$defs":"definitions";if(e.external){let m=e.external.registry.get(s[0])?.id,d=e.external.uri??(g=>g);if(m)return{ref:d(m)};let f=s[1].defId??s[1].schema.id??`schema${e.counter++}`;return s[1].defId=f,{defId:f,ref:`${d("__shared")}#/${a}/${f}`}}if(s[1]===n)return{ref:"#"};let l=`#/${a}/`,p=s[1].schema.id??`__schema${e.counter++}`;return{defId:p,ref:l+p}},i=s=>{if(s[1].schema.$ref)return;let a=s[1],{ref:u,defId:l}=o(s);a.def={...a.schema},l&&(a.defId=l);let p=a.schema;for(let m in p)delete p[m];p.$ref=u};if(e.cycles==="throw")for(let s of e.seen.entries()){let a=s[1];if(a.cycle)throw new Error(`Cycle detected: #/${a.cycle?.join("/")}/<root>

Set the \`cycles\` parameter to \`"ref"\` to resolve cyclical schemas with defs.`)}for(let s of e.seen.entries()){let a=s[1];if(t===s[0]){i(s);continue}if(e.external){let l=e.external.registry.get(s[0])?.id;if(t!==s[0]&&l){i(s);continue}}if(e.metadataRegistry.get(s[0])?.id){i(s);continue}if(a.cycle){i(s);continue}if(a.count>1&&e.reused==="ref"){i(s);continue}}}function xt(e,t){let n=e.seen.get(t);if(!n)throw new Error("Unprocessed schema. This is a bug in Zod.");let r=a=>{let u=e.seen.get(a);if(u.ref===null)return;let l=u.def??u.schema,p={...l},m=u.ref;if(u.ref=null,m){r(m);let f=e.seen.get(m),g=f.schema;if(g.$ref&&(e.target==="draft-07"||e.target==="draft-04"||e.target==="openapi-3.0")?(l.allOf=l.allOf??[],l.allOf.push(g)):Object.assign(l,g),Object.assign(l,p),a._zod.parent===m)for(let w in l)w==="$ref"||w==="allOf"||w in p||delete l[w];if(g.$ref&&f.def)for(let w in l)w==="$ref"||w==="allOf"||w in f.def&&JSON.stringify(l[w])===JSON.stringify(f.def[w])&&delete l[w]}let d=a._zod.parent;if(d&&d!==m){r(d);let f=e.seen.get(d);if(f?.schema.$ref&&(l.$ref=f.schema.$ref,f.def))for(let g in l)g==="$ref"||g==="allOf"||g in f.def&&JSON.stringify(l[g])===JSON.stringify(f.def[g])&&delete l[g]}e.override({zodSchema:a,jsonSchema:l,path:u.path??[]})};for(let a of[...e.seen.entries()].reverse())r(a[0]);let o={};if(e.target==="draft-2020-12"?o.$schema="https://json-schema.org/draft/2020-12/schema":e.target==="draft-07"?o.$schema="http://json-schema.org/draft-07/schema#":e.target==="draft-04"?o.$schema="http://json-schema.org/draft-04/schema#":e.target,e.external?.uri){let a=e.external.registry.get(t)?.id;if(!a)throw new Error("Schema is missing an `id` property");o.$id=e.external.uri(a)}Object.assign(o,n.def??n.schema);let i=e.metadataRegistry.get(t)?.id;i!==void 0&&o.id===i&&delete o.id;let s=e.external?.defs??{};for(let a of e.seen.entries()){let u=a[1];u.def&&u.defId&&(u.def.id===u.defId&&delete u.def.id,s[u.defId]=u.def)}e.external||Object.keys(s).length>0&&(e.target==="draft-2020-12"?o.$defs=s:o.definitions=s);try{let a=JSON.parse(JSON.stringify(o));return Object.defineProperty(a,"~standard",{value:{...t["~standard"],jsonSchema:{input:gt(t,"input",e.processors),output:gt(t,"output",e.processors)}},enumerable:!1,writable:!1}),a}catch{throw new Error("Error converting schema to JSON.")}}function ie(e,t){let n=t??{seen:new Set};if(n.seen.has(e))return!1;n.seen.add(e);let r=e._zod.def;if(r.type==="transform")return!0;if(r.type==="array")return ie(r.element,n);if(r.type==="set")return ie(r.valueType,n);if(r.type==="lazy")return ie(r.getter(),n);if(r.type==="promise"||r.type==="optional"||r.type==="nonoptional"||r.type==="nullable"||r.type==="readonly"||r.type==="default"||r.type==="prefault")return ie(r.innerType,n);if(r.type==="intersection")return ie(r.left,n)||ie(r.right,n);if(r.type==="record"||r.type==="map")return ie(r.keyType,n)||ie(r.valueType,n);if(r.type==="pipe")return e._zod.traits.has("$ZodCodec")?!0:ie(r.in,n)||ie(r.out,n);if(r.type==="object"){for(let o in r.shape)if(ie(r.shape[o],n))return!0;return!1}if(r.type==="union"){for(let o of r.options)if(ie(o,n))return!0;return!1}if(r.type==="tuple"){for(let o of r.items)if(ie(o,n))return!0;return!!(r.rest&&ie(r.rest,n))}return!1}var is=(e,t={})=>n=>{let r=_t({...n,processors:t});return F(e,r),bt(r,e),xt(r,e)},gt=(e,t,n={})=>r=>{let{libraryOptions:o,target:i}=r??{},s=_t({...o??{},target:i,io:t,processors:n});return F(e,s),bt(s,e),xt(s,e)};var mc={guid:"uuid",url:"uri",datetime:"date-time",json_string:"json-string",regex:""},In=(e,t,n,r)=>{let o=n;o.type="string";let{minimum:i,maximum:s,format:a,patterns:u,contentEncoding:l}=e._zod.bag;if(typeof i=="number"&&(o.minLength=i),typeof s=="number"&&(o.maxLength=s),a&&(o.format=mc[a]??a,o.format===""&&delete o.format,a==="time"&&delete o.format),l&&(o.contentEncoding=l),u&&u.size>0){let p=[...u];p.length===1?o.pattern=p[0].source:p.length>1&&(o.allOf=[...p.map(m=>({...t.target==="draft-07"||t.target==="draft-04"||t.target==="openapi-3.0"?{type:"string"}:{},pattern:m.source}))])}},On=(e,t,n,r)=>{let o=n,{minimum:i,maximum:s,format:a,multipleOf:u,exclusiveMaximum:l,exclusiveMinimum:p}=e._zod.bag;typeof a=="string"&&a.includes("int")?o.type="integer":o.type="number";let m=typeof p=="number"&&p>=(i??Number.NEGATIVE_INFINITY),d=typeof l=="number"&&l<=(s??Number.POSITIVE_INFINITY),f=t.target==="draft-04"||t.target==="openapi-3.0";m?f?(o.minimum=p,o.exclusiveMinimum=!0):o.exclusiveMinimum=p:typeof i=="number"&&(o.minimum=i),d?f?(o.maximum=l,o.exclusiveMaximum=!0):o.exclusiveMaximum=l:typeof s=="number"&&(o.maximum=s),typeof u=="number"&&(o.multipleOf=u)},Nn=(e,t,n,r)=>{n.type="boolean"},as=(e,t,n,r)=>{if(t.unrepresentable==="throw")throw new Error("BigInt cannot be represented in JSON Schema")},cs=(e,t,n,r)=>{if(t.unrepresentable==="throw")throw new Error("Symbols cannot be represented in JSON Schema")},us=(e,t,n,r)=>{t.target==="openapi-3.0"?(n.type="string",n.nullable=!0,n.enum=[null]):n.type="null"},ls=(e,t,n,r)=>{if(t.unrepresentable==="throw")throw new Error("Undefined cannot be represented in JSON Schema")},ps=(e,t,n,r)=>{if(t.unrepresentable==="throw")throw new Error("Void cannot be represented in JSON Schema")},Cn=(e,t,n,r)=>{n.not={}},ds=(e,t,n,r)=>{},Zn=(e,t,n,r)=>{},hs=(e,t,n,r)=>{if(t.unrepresentable==="throw")throw new Error("Date cannot be represented in JSON Schema")},Rn=(e,t,n,r)=>{let o=e._zod.def,i=rt(o.entries);i.every(s=>typeof s=="number")&&(n.type="number"),i.every(s=>typeof s=="string")&&(n.type="string"),n.enum=i},fs=(e,t,n,r)=>{let o=e._zod.def,i=[];for(let s of o.values)if(s===void 0){if(t.unrepresentable==="throw")throw new Error("Literal `undefined` cannot be represented in JSON Schema")}else if(typeof s=="bigint"){if(t.unrepresentable==="throw")throw new Error("BigInt literals cannot be represented in JSON Schema");i.push(Number(s))}else i.push(s);if(i.length!==0)if(i.length===1){let s=i[0];n.type=s===null?"null":typeof s,t.target==="draft-04"||t.target==="openapi-3.0"?n.enum=[s]:n.const=s}else i.every(s=>typeof s=="number")&&(n.type="number"),i.every(s=>typeof s=="string")&&(n.type="string"),i.every(s=>typeof s=="boolean")&&(n.type="boolean"),i.every(s=>s===null)&&(n.type="null"),n.enum=i},ms=(e,t,n,r)=>{if(t.unrepresentable==="throw")throw new Error("NaN cannot be represented in JSON Schema")},gs=(e,t,n,r)=>{let o=n,i=e._zod.pattern;if(!i)throw new Error("Pattern not found in template literal");o.type="string",o.pattern=i.source},_s=(e,t,n,r)=>{let o=n,i={type:"string",format:"binary",contentEncoding:"binary"},{minimum:s,maximum:a,mime:u}=e._zod.bag;s!==void 0&&(i.minLength=s),a!==void 0&&(i.maxLength=a),u?u.length===1?(i.contentMediaType=u[0],Object.assign(o,i)):(Object.assign(o,i),o.anyOf=u.map(l=>({contentMediaType:l}))):Object.assign(o,i)},bs=(e,t,n,r)=>{n.type="boolean"},Ln=(e,t,n,r)=>{if(t.unrepresentable==="throw")throw new Error("Custom types cannot be represented in JSON Schema")},xs=(e,t,n,r)=>{if(t.unrepresentable==="throw")throw new Error("Function types cannot be represented in JSON Schema")},Mn=(e,t,n,r)=>{if(t.unrepresentable==="throw")throw new Error("Transforms cannot be represented in JSON Schema")},ys=(e,t,n,r)=>{if(t.unrepresentable==="throw")throw new Error("Map cannot be represented in JSON Schema")},ws=(e,t,n,r)=>{if(t.unrepresentable==="throw")throw new Error("Set cannot be represented in JSON Schema")},Fn=(e,t,n,r)=>{let o=n,i=e._zod.def,{minimum:s,maximum:a}=e._zod.bag;typeof s=="number"&&(o.minItems=s),typeof a=="number"&&(o.maxItems=a),o.type="array",o.items=F(i.element,t,{...r,path:[...r.path,"items"]})},Dn=(e,t,n,r)=>{let o=n,i=e._zod.def;o.type="object",o.properties={};let s=i.shape;for(let l in s)o.properties[l]=F(s[l],t,{...r,path:[...r.path,"properties",l]});let a=new Set(Object.keys(s)),u=new Set([...a].filter(l=>{let p=i.shape[l]._zod;return t.io==="input"?p.optin===void 0:p.optout===void 0}));u.size>0&&(o.required=Array.from(u)),i.catchall?._zod.def.type==="never"?o.additionalProperties=!1:i.catchall?i.catchall&&(o.additionalProperties=F(i.catchall,t,{...r,path:[...r.path,"additionalProperties"]})):t.io==="output"&&(o.additionalProperties=!1)},jn=(e,t,n,r)=>{let o=e._zod.def,i=o.inclusive===!1,s=o.options.map((a,u)=>F(a,t,{...r,path:[...r.path,i?"oneOf":"anyOf",u]}));i?n.oneOf=s:n.anyOf=s},Un=(e,t,n,r)=>{let o=e._zod.def,i=F(o.left,t,{...r,path:[...r.path,"allOf",0]}),s=F(o.right,t,{...r,path:[...r.path,"allOf",1]}),a=l=>"allOf"in l&&Object.keys(l).length===1,u=[...a(i)?i.allOf:[i],...a(s)?s.allOf:[s]];n.allOf=u},vs=(e,t,n,r)=>{let o=n,i=e._zod.def;o.type="array";let s=t.target==="draft-2020-12"?"prefixItems":"items",a=t.target==="draft-2020-12"||t.target==="openapi-3.0"?"items":"additionalItems",u=i.items.map((d,f)=>F(d,t,{...r,path:[...r.path,s,f]})),l=i.rest?F(i.rest,t,{...r,path:[...r.path,a,...t.target==="openapi-3.0"?[i.items.length]:[]]}):null;t.target==="draft-2020-12"?(o.prefixItems=u,l&&(o.items=l)):t.target==="openapi-3.0"?(o.items={anyOf:u},l&&o.items.anyOf.push(l),o.minItems=u.length,l||(o.maxItems=u.length)):(o.items=u,l&&(o.additionalItems=l));let{minimum:p,maximum:m}=e._zod.bag;typeof p=="number"&&(o.minItems=p),typeof m=="number"&&(o.maxItems=m)},ks=(e,t,n,r)=>{let o=n,i=e._zod.def;o.type="object";let s=i.keyType,u=s._zod.bag?.patterns;if(i.mode==="loose"&&u&&u.size>0){let p=F(i.valueType,t,{...r,path:[...r.path,"patternProperties","*"]});o.patternProperties={};for(let m of u)o.patternProperties[m.source]=p}else(t.target==="draft-07"||t.target==="draft-2020-12")&&(o.propertyNames=F(i.keyType,t,{...r,path:[...r.path,"propertyNames"]})),o.additionalProperties=F(i.valueType,t,{...r,path:[...r.path,"additionalProperties"]});let l=s._zod.values;if(l){let p=[...l].filter(m=>typeof m=="string"||typeof m=="number");p.length>0&&(o.required=p)}},Bn=(e,t,n,r)=>{let o=e._zod.def,i=F(o.innerType,t,r),s=t.seen.get(e);t.target==="openapi-3.0"?(s.ref=o.innerType,n.nullable=!0):n.anyOf=[i,{type:"null"}]},Wn=(e,t,n,r)=>{let o=e._zod.def;F(o.innerType,t,r);let i=t.seen.get(e);i.ref=o.innerType},Vn=(e,t,n,r)=>{let o=e._zod.def;F(o.innerType,t,r);let i=t.seen.get(e);i.ref=o.innerType,n.default=JSON.parse(JSON.stringify(o.defaultValue))},Gn=(e,t,n,r)=>{let o=e._zod.def;F(o.innerType,t,r);let i=t.seen.get(e);i.ref=o.innerType,t.io==="input"&&(n._prefault=JSON.parse(JSON.stringify(o.defaultValue)))},Jn=(e,t,n,r)=>{let o=e._zod.def;F(o.innerType,t,r);let i=t.seen.get(e);i.ref=o.innerType;let s;try{s=o.catchValue(void 0)}catch{throw new Error("Dynamic catch values are not supported in JSON Schema")}n.default=s},Hn=(e,t,n,r)=>{let o=e._zod.def,i=o.in._zod.traits.has("$ZodTransform"),s=t.io==="input"?i?o.out:o.in:o.out;F(s,t,r);let a=t.seen.get(e);a.ref=s},qn=(e,t,n,r)=>{let o=e._zod.def;F(o.innerType,t,r);let i=t.seen.get(e);i.ref=o.innerType,n.readOnly=!0},$s=(e,t,n,r)=>{let o=e._zod.def;F(o.innerType,t,r);let i=t.seen.get(e);i.ref=o.innerType},Bt=(e,t,n,r)=>{let o=e._zod.def;F(o.innerType,t,r);let i=t.seen.get(e);i.ref=o.innerType},zs=(e,t,n,r)=>{let o=e._zod.innerType;F(o,t,r);let i=t.seen.get(e);i.ref=o},ss={string:In,number:On,boolean:Nn,bigint:as,symbol:cs,null:us,undefined:ls,void:ps,never:Cn,any:ds,unknown:Zn,date:hs,enum:Rn,literal:fs,nan:ms,template_literal:gs,file:_s,success:bs,custom:Ln,function:xs,transform:Mn,map:ys,set:ws,array:Fn,object:Dn,union:jn,intersection:Un,tuple:vs,record:ks,nullable:Bn,nonoptional:Wn,default:Vn,prefault:Gn,catch:Jn,pipe:Hn,readonly:qn,promise:$s,optional:Bt,lazy:zs};function Yn(e,t){if("_idmap"in e){let r=e,o=_t({...t,processors:ss}),i={};for(let u of r._idmap.entries()){let[l,p]=u;F(p,o)}let s={},a={registry:r,uri:t?.uri,defs:i};o.external=a;for(let u of r._idmap.entries()){let[l,p]=u;bt(o,p),s[l]=xt(o,p)}if(Object.keys(i).length>0){let u=o.target==="draft-2020-12"?"$defs":"definitions";s.__shared={[u]:i}}return{schemas:s}}let n=_t({...t,processors:ss});return F(e,n),bt(n,e),xt(n,e)}var kc=h("ZodISODateTime",(e,t)=>{Uo.init(e,t),U.init(e,t)});function Es(e){return Ji(kc,e)}var $c=h("ZodISODate",(e,t)=>{Bo.init(e,t),U.init(e,t)});function Ss(e){return Hi($c,e)}var zc=h("ZodISOTime",(e,t)=>{Wo.init(e,t),U.init(e,t)});function Ts(e){return qi(zc,e)}var Ec=h("ZodISODuration",(e,t)=>{Vo.init(e,t),U.init(e,t)});function Ps(e){return Yi(Ec,e)}var Tc=(e,t)=>{It.init(e,t),e.name="ZodError",Object.defineProperties(e,{format:{value:n=>ln(e,n)},flatten:{value:n=>un(e,n)},addIssue:{value:n=>{e.issues.push(n),e.message=JSON.stringify(e.issues,We,2)}},addIssues:{value:n=>{e.issues.push(...n),e.message=JSON.stringify(e.issues,We,2)}},isEmpty:{get(){return e.issues.length===0}}})};var ce=h("ZodError",Tc,{Parent:Error});var Is=Ot(ce),Os=Nt(ce),Ns=lt(ce),Cs=pt(ce),Zs=zr(ce),Rs=Er(ce),Ls=Sr(ce),Ms=Tr(ce),Fs=Pr(ce),Ds=Ar(ce),js=Ir(ce),Us=Or(ce);var Bs=new WeakMap;function wt(e,t,n){let r=Object.getPrototypeOf(e),o=Bs.get(r);if(o||(o=new Set,Bs.set(r,o)),!o.has(t)){o.add(t);for(let i in n){let s=n[i];Object.defineProperty(r,i,{configurable:!0,enumerable:!1,get(){let a=s.bind(this);return Object.defineProperty(this,i,{configurable:!0,writable:!0,enumerable:!0,value:a}),a},set(a){Object.defineProperty(this,i,{configurable:!0,writable:!0,enumerable:!0,value:a})}})}}}var H=h("ZodType",(e,t)=>(V.init(e,t),Object.assign(e["~standard"],{jsonSchema:{input:gt(e,"input"),output:gt(e,"output")}}),e.toJSONSchema=is(e,{}),e.def=t,e.type=t.type,Object.defineProperty(e,"_def",{value:t}),e.parse=(n,r)=>Is(e,n,r,{callee:e.parse}),e.safeParse=(n,r)=>Ns(e,n,r),e.parseAsync=async(n,r)=>Os(e,n,r,{callee:e.parseAsync}),e.safeParseAsync=async(n,r)=>Cs(e,n,r),e.spa=e.safeParseAsync,e.encode=(n,r)=>Zs(e,n,r),e.decode=(n,r)=>Rs(e,n,r),e.encodeAsync=async(n,r)=>Ls(e,n,r),e.decodeAsync=async(n,r)=>Ms(e,n,r),e.safeEncode=(n,r)=>Fs(e,n,r),e.safeDecode=(n,r)=>Ds(e,n,r),e.safeEncodeAsync=async(n,r)=>js(e,n,r),e.safeDecodeAsync=async(n,r)=>Us(e,n,r),wt(e,"ZodType",{check(...n){let r=this.def;return this.clone(Z.mergeDefs(r,{checks:[...r.checks??[],...n.map(o=>typeof o=="function"?{_zod:{check:o,def:{check:"custom"},onattach:[]}}:o)]}),{parent:!0})},with(...n){return this.check(...n)},clone(n,r){return pe(this,n,r)},brand(){return this},register(n,r){return n.add(this,r),this},refine(n,r){return this.check(wu(n,r))},superRefine(n,r){return this.check(vu(n,r))},overwrite(n){return this.check(Ae(n))},optional(){return Gs(this)},exactOptional(){return cu(this)},nullable(){return Js(this)},nullish(){return Gs(Js(this))},nonoptional(n){return fu(this,n)},array(){return eu(this)},or(n){return Qn([this,n])},and(n){return ru(this,n)},transform(n){return Hs(this,su(n))},default(n){return pu(this,n)},prefault(n){return hu(this,n)},catch(n){return gu(this,n)},pipe(n){return Hs(this,n)},readonly(){return xu(this)},describe(n){let r=this.clone();return Pe.add(r,{description:n}),r},meta(...n){if(n.length===0)return Pe.get(this);let r=this.clone();return Pe.add(r,n[0]),r},isOptional(){return this.safeParse(void 0).success},isNullable(){return this.safeParse(null).success},apply(n){return n(this)}}),Object.defineProperty(e,"description",{get(){return Pe.get(e)?.description},configurable:!0}),e)),qs=h("_ZodString",(e,t)=>{Lt.init(e,t),H.init(e,t),e._zod.processJSONSchema=(r,o,i)=>In(e,r,o,i);let n=e._zod.bag;e.format=n.format??null,e.minLength=n.minimum??null,e.maxLength=n.maximum??null,wt(e,"_ZodString",{regex(...r){return this.check(yn(...r))},includes(...r){return this.check(kn(...r))},startsWith(...r){return this.check($n(...r))},endsWith(...r){return this.check(zn(...r))},min(...r){return this.check(Ge(...r))},max(...r){return this.check(jt(...r))},length(...r){return this.check(Ut(...r))},nonempty(...r){return this.check(Ge(1,...r))},lowercase(r){return this.check(wn(r))},uppercase(r){return this.check(vn(r))},trim(){return this.check(Sn())},normalize(...r){return this.check(En(...r))},toLowerCase(){return this.check(Tn())},toUpperCase(){return this.check(Pn())},slugify(){return this.check(An())}})}),Ac=h("ZodString",(e,t)=>{Lt.init(e,t),qs.init(e,t),e.email=n=>e.check(Ei(Ic,n)),e.url=n=>e.check(Ii(Oc,n)),e.jwt=n=>e.check(Gi(Jc,n)),e.emoji=n=>e.check(Oi(Nc,n)),e.guid=n=>e.check(xn(Ws,n)),e.uuid=n=>e.check(Si(Wt,n)),e.uuidv4=n=>e.check(Ti(Wt,n)),e.uuidv6=n=>e.check(Pi(Wt,n)),e.uuidv7=n=>e.check(Ai(Wt,n)),e.nanoid=n=>e.check(Ni(Cc,n)),e.guid=n=>e.check(xn(Ws,n)),e.cuid=n=>e.check(Ci(Zc,n)),e.cuid2=n=>e.check(Zi(Rc,n)),e.ulid=n=>e.check(Ri(Lc,n)),e.base64=n=>e.check(Bi(Wc,n)),e.base64url=n=>e.check(Wi(Vc,n)),e.xid=n=>e.check(Li(Mc,n)),e.ksuid=n=>e.check(Mi(Fc,n)),e.ipv4=n=>e.check(Fi(Dc,n)),e.ipv6=n=>e.check(Di(jc,n)),e.cidrv4=n=>e.check(ji(Uc,n)),e.cidrv6=n=>e.check(Ui(Bc,n)),e.e164=n=>e.check(Vi(Gc,n)),e.datetime=n=>e.check(Es(n)),e.date=n=>e.check(Ss(n)),e.time=n=>e.check(Ts(n)),e.duration=n=>e.check(Ps(n))});function $e(e){return zi(Ac,e)}var U=h("ZodStringFormat",(e,t)=>{j.init(e,t),qs.init(e,t)}),Ic=h("ZodEmail",(e,t)=>{No.init(e,t),U.init(e,t)});var Ws=h("ZodGUID",(e,t)=>{Io.init(e,t),U.init(e,t)});var Wt=h("ZodUUID",(e,t)=>{Oo.init(e,t),U.init(e,t)});var Oc=h("ZodURL",(e,t)=>{Co.init(e,t),U.init(e,t)});var Nc=h("ZodEmoji",(e,t)=>{Zo.init(e,t),U.init(e,t)});var Cc=h("ZodNanoID",(e,t)=>{Ro.init(e,t),U.init(e,t)});var Zc=h("ZodCUID",(e,t)=>{Lo.init(e,t),U.init(e,t)});var Rc=h("ZodCUID2",(e,t)=>{Mo.init(e,t),U.init(e,t)});var Lc=h("ZodULID",(e,t)=>{Fo.init(e,t),U.init(e,t)});var Mc=h("ZodXID",(e,t)=>{Do.init(e,t),U.init(e,t)});var Fc=h("ZodKSUID",(e,t)=>{jo.init(e,t),U.init(e,t)});var Dc=h("ZodIPv4",(e,t)=>{Go.init(e,t),U.init(e,t)});var jc=h("ZodIPv6",(e,t)=>{Jo.init(e,t),U.init(e,t)});var Uc=h("ZodCIDRv4",(e,t)=>{Ho.init(e,t),U.init(e,t)});var Bc=h("ZodCIDRv6",(e,t)=>{qo.init(e,t),U.init(e,t)});var Wc=h("ZodBase64",(e,t)=>{Xo.init(e,t),U.init(e,t)});var Vc=h("ZodBase64URL",(e,t)=>{Ko.init(e,t),U.init(e,t)});var Gc=h("ZodE164",(e,t)=>{Qo.init(e,t),U.init(e,t)});var Jc=h("ZodJWT",(e,t)=>{ei.init(e,t),U.init(e,t)});var Ys=h("ZodNumber",(e,t)=>{gn.init(e,t),H.init(e,t),e._zod.processJSONSchema=(r,o,i)=>On(e,r,o,i),wt(e,"ZodNumber",{gt(r,o){return this.check(Ft(r,o))},gte(r,o){return this.check(mt(r,o))},min(r,o){return this.check(mt(r,o))},lt(r,o){return this.check(Mt(r,o))},lte(r,o){return this.check(ft(r,o))},max(r,o){return this.check(ft(r,o))},int(r){return this.check(Je(r))},safe(r){return this.check(Je(r))},positive(r){return this.check(Ft(0,r))},nonnegative(r){return this.check(mt(0,r))},negative(r){return this.check(Mt(0,r))},nonpositive(r){return this.check(ft(0,r))},multipleOf(r,o){return this.check(Dt(r,o))},step(r,o){return this.check(Dt(r,o))},finite(){return this}});let n=e._zod.bag;e.minValue=Math.max(n.minimum??Number.NEGATIVE_INFINITY,n.exclusiveMinimum??Number.NEGATIVE_INFINITY)??null,e.maxValue=Math.min(n.maximum??Number.POSITIVE_INFINITY,n.exclusiveMaximum??Number.POSITIVE_INFINITY)??null,e.isInt=(n.format??"").includes("int")||Number.isSafeInteger(n.multipleOf??.5),e.isFinite=!0,e.format=n.format??null});function Re(e){return Xi(Ys,e)}var Hc=h("ZodNumberFormat",(e,t)=>{ti.init(e,t),Ys.init(e,t)});function Je(e){return Ki(Hc,e)}var qc=h("ZodBoolean",(e,t)=>{ni.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Nn(e,n,r,o)});function Vt(e){return Qi(qc,e)}var Yc=h("ZodUnknown",(e,t)=>{ri.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Zn(e,n,r,o)});function Vs(){return es(Yc)}var Xc=h("ZodNever",(e,t)=>{oi.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Cn(e,n,r,o)});function Kc(e){return ts(Xc,e)}var Qc=h("ZodArray",(e,t)=>{ii.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Fn(e,n,r,o),e.element=t.element,wt(e,"ZodArray",{min(n,r){return this.check(Ge(n,r))},nonempty(n){return this.check(Ge(1,n))},max(n,r){return this.check(jt(n,r))},length(n,r){return this.check(Ut(n,r))},unwrap(){return this.element}})});function eu(e,t){return ns(Qc,e,t)}var Kn=h("ZodObject",(e,t)=>{ci.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Dn(e,n,r,o),Z.defineLazy(e,"shape",()=>t.shape),wt(e,"ZodObject",{keyof(){return ou(Object.keys(this._zod.def.shape))},catchall(n){return this.clone({...this._zod.def,catchall:n})},passthrough(){return this.clone({...this._zod.def,catchall:Vs()})},loose(){return this.clone({...this._zod.def,catchall:Vs()})},strict(){return this.clone({...this._zod.def,catchall:Kc()})},strip(){return this.clone({...this._zod.def,catchall:void 0})},extend(n){return Z.extend(this,n)},safeExtend(n){return Z.safeExtend(this,n)},merge(n){return Z.merge(this,n)},pick(n){return Z.pick(this,n)},omit(n){return Z.omit(this,n)},partial(...n){return Z.partial(Xs,this,n[0])},required(...n){return Z.required(Ks,this,n[0])}})});function de(e,t){let n={type:"object",shape:e??{},...Z.normalizeParams(t)};return new Kn(n)}var tu=h("ZodUnion",(e,t)=>{ui.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>jn(e,n,r,o),e.options=t.options});function Qn(e,t){return new tu({type:"union",options:e,...Z.normalizeParams(t)})}var nu=h("ZodIntersection",(e,t)=>{li.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Un(e,n,r,o)});function ru(e,t){return new nu({type:"intersection",left:e,right:t})}var Xn=h("ZodEnum",(e,t)=>{pi.init(e,t),H.init(e,t),e._zod.processJSONSchema=(r,o,i)=>Rn(e,r,o,i),e.enum=t.entries,e.options=Object.values(t.entries);let n=new Set(Object.keys(t.entries));e.extract=(r,o)=>{let i={};for(let s of r)if(n.has(s))i[s]=t.entries[s];else throw new Error(`Key ${s} not found in enum`);return new Xn({...t,checks:[],...Z.normalizeParams(o),entries:i})},e.exclude=(r,o)=>{let i={...t.entries};for(let s of r)if(n.has(s))delete i[s];else throw new Error(`Key ${s} not found in enum`);return new Xn({...t,checks:[],...Z.normalizeParams(o),entries:i})}});function ou(e,t){let n=Array.isArray(e)?Object.fromEntries(e.map(r=>[r,r])):e;return new Xn({type:"enum",entries:n,...Z.normalizeParams(t)})}var iu=h("ZodTransform",(e,t)=>{di.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Mn(e,n,r,o),e._zod.parse=(n,r)=>{if(r.direction==="backward")throw new je(e.constructor.name);n.addIssue=i=>{if(typeof i=="string")n.issues.push(Z.issue(i,n.value,t));else{let s=i;s.fatal&&(s.continue=!1),s.code??(s.code="custom"),s.input??(s.input=n.value),s.inst??(s.inst=e),n.issues.push(Z.issue(s))}};let o=t.transform(n.value,n);return o instanceof Promise?o.then(i=>(n.value=i,n.fallback=!0,n)):(n.value=o,n.fallback=!0,n)}});function su(e){return new iu({type:"transform",transform:e})}var Xs=h("ZodOptional",(e,t)=>{_n.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Bt(e,n,r,o),e.unwrap=()=>e._zod.def.innerType});function Gs(e){return new Xs({type:"optional",innerType:e})}var au=h("ZodExactOptional",(e,t)=>{hi.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Bt(e,n,r,o),e.unwrap=()=>e._zod.def.innerType});function cu(e){return new au({type:"optional",innerType:e})}var uu=h("ZodNullable",(e,t)=>{fi.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Bn(e,n,r,o),e.unwrap=()=>e._zod.def.innerType});function Js(e){return new uu({type:"nullable",innerType:e})}var lu=h("ZodDefault",(e,t)=>{mi.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Vn(e,n,r,o),e.unwrap=()=>e._zod.def.innerType,e.removeDefault=e.unwrap});function pu(e,t){return new lu({type:"default",innerType:e,get defaultValue(){return typeof t=="function"?t():Z.shallowClone(t)}})}var du=h("ZodPrefault",(e,t)=>{gi.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Gn(e,n,r,o),e.unwrap=()=>e._zod.def.innerType});function hu(e,t){return new du({type:"prefault",innerType:e,get defaultValue(){return typeof t=="function"?t():Z.shallowClone(t)}})}var Ks=h("ZodNonOptional",(e,t)=>{_i.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Wn(e,n,r,o),e.unwrap=()=>e._zod.def.innerType});function fu(e,t){return new Ks({type:"nonoptional",innerType:e,...Z.normalizeParams(t)})}var mu=h("ZodCatch",(e,t)=>{bi.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Jn(e,n,r,o),e.unwrap=()=>e._zod.def.innerType,e.removeCatch=e.unwrap});function gu(e,t){return new mu({type:"catch",innerType:e,catchValue:typeof t=="function"?t:()=>t})}var _u=h("ZodPipe",(e,t)=>{xi.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Hn(e,n,r,o),e.in=t.in,e.out=t.out});function Hs(e,t){return new _u({type:"pipe",in:e,out:t})}var bu=h("ZodReadonly",(e,t)=>{yi.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>qn(e,n,r,o),e.unwrap=()=>e._zod.def.innerType});function xu(e){return new bu({type:"readonly",innerType:e})}var yu=h("ZodCustom",(e,t)=>{wi.init(e,t),H.init(e,t),e._zod.processJSONSchema=(n,r,o)=>Ln(e,n,r,o)});function wu(e,t={}){return rs(yu,e,t)}function vu(e,t){return os(e,t)}le(vi());var Qs=(e=0)=>t=>`\x1B[${t+e}m`,ea=(e=0)=>t=>`\x1B[${38+e};5;${t}m`,ta=(e=0)=>(t,n,r)=>`\x1B[${38+e};2;${t};${n};${r}m`,B={modifier:{reset:[0,0],bold:[1,22],dim:[2,22],italic:[3,23],underline:[4,24],overline:[53,55],inverse:[7,27],hidden:[8,28],strikethrough:[9,29]},color:{black:[30,39],red:[31,39],green:[32,39],yellow:[33,39],blue:[34,39],magenta:[35,39],cyan:[36,39],white:[37,39],blackBright:[90,39],gray:[90,39],grey:[90,39],redBright:[91,39],greenBright:[92,39],yellowBright:[93,39],blueBright:[94,39],magentaBright:[95,39],cyanBright:[96,39],whiteBright:[97,39]},bgColor:{bgBlack:[40,49],bgRed:[41,49],bgGreen:[42,49],bgYellow:[43,49],bgBlue:[44,49],bgMagenta:[45,49],bgCyan:[46,49],bgWhite:[47,49],bgBlackBright:[100,49],bgGray:[100,49],bgGrey:[100,49],bgRedBright:[101,49],bgGreenBright:[102,49],bgYellowBright:[103,49],bgBlueBright:[104,49],bgMagentaBright:[105,49],bgCyanBright:[106,49],bgWhiteBright:[107,49]}},Gd=Object.keys(B.modifier),ku=Object.keys(B.color),$u=Object.keys(B.bgColor),Jd=[...ku,...$u];function zu(){let e=new Map;for(let[t,n]of Object.entries(B)){for(let[r,o]of Object.entries(n))B[r]={open:`\x1B[${o[0]}m`,close:`\x1B[${o[1]}m`},n[r]=B[r],e.set(o[0],o[1]);Object.defineProperty(B,t,{value:n,enumerable:!1})}return Object.defineProperty(B,"codes",{value:e,enumerable:!1}),B.color.close="\x1B[39m",B.bgColor.close="\x1B[49m",B.color.ansi=Qs(),B.color.ansi256=ea(),B.color.ansi16m=ta(),B.bgColor.ansi=Qs(10),B.bgColor.ansi256=ea(10),B.bgColor.ansi16m=ta(10),Object.defineProperties(B,{rgbToAnsi256:{value(t,n,r){return t===n&&n===r?t<8?16:t>248?231:Math.round((t-8)/247*24)+232:16+36*Math.round(t/255*5)+6*Math.round(n/255*5)+Math.round(r/255*5)},enumerable:!1},hexToRgb:{value(t){let n=/[a-f\d]{6}|[a-f\d]{3}/i.exec(t.toString(16));if(!n)return[0,0,0];let[r]=n;r.length===3&&(r=[...r].map(i=>i+i).join(""));let o=Number.parseInt(r,16);return[o>>16&255,o>>8&255,o&255]},enumerable:!1},hexToAnsi256:{value:t=>B.rgbToAnsi256(...B.hexToRgb(t)),enumerable:!1},ansi256ToAnsi:{value(t){if(t<8)return 30+t;if(t<16)return 90+(t-8);let n,r,o;if(t>=232)n=((t-232)*10+8)/255,r=n,o=n;else{t-=16;let a=t%36;n=Math.floor(t/36)/5,r=Math.floor(a/6)/5,o=a%6/5}let i=Math.max(n,r,o)*2;if(i===0)return 30;let s=30+(Math.round(o)<<2|Math.round(r)<<1|Math.round(n));return i===2&&(s+=60),s},enumerable:!1},rgbToAnsi:{value:(t,n,r)=>B.ansi256ToAnsi(B.rgbToAnsi256(t,n,r)),enumerable:!1},hexToAnsi:{value:t=>B.ansi256ToAnsi(B.hexToAnsi256(t)),enumerable:!1}}),B}var Eu=zu(),be=Eu;var Gt=(()=>{if(!("navigator"in globalThis))return 0;if(globalThis.navigator.userAgentData){let e=navigator.userAgentData.brands.find(({brand:t})=>t==="Chromium");if(e&&e.version>93)return 3}return/\b(Chrome|Chromium)\//.test(globalThis.navigator.userAgent)?1:0})(),na=Gt!==0&&{level:Gt,hasBasic:!0,has256:Gt>=2,has16m:Gt>=3},Su={stdout:na,stderr:na},ra=Su;function oa(e,t,n){let r=e.indexOf(t);if(r===-1)return e;let o=t.length,i=0,s="";do s+=e.slice(i,r)+t+n,i=r+o,r=e.indexOf(t,i);while(r!==-1);return s+=e.slice(i),s}function ia(e,t,n,r){let o=0,i="";do{let s=e[r-1]==="\r";i+=e.slice(o,s?r-1:r)+t+(s?`\r
`:`
`)+n,o=r+1,r=e.indexOf(`
`,o)}while(r!==-1);return i+=e.slice(o),i}var{stdout:sa,stderr:aa}=ra,er=Symbol("GENERATOR"),He=Symbol("STYLER"),vt=Symbol("IS_EMPTY"),ca=["ansi","ansi","ansi256","ansi16m"],qe=Object.create(null),Tu=(e,t={})=>{if(t.level&&!(Number.isInteger(t.level)&&t.level>=0&&t.level<=3))throw new Error("The `level` option should be an integer from 0 to 3");let n=sa?sa.level:0;e.level=t.level===void 0?n:t.level};var Pu=e=>{let t=(...n)=>n.join(" ");return Tu(t,e),Object.setPrototypeOf(t,kt.prototype),t};function kt(e){return Pu(e)}Object.setPrototypeOf(kt.prototype,Function.prototype);for(let[e,t]of Object.entries(be))qe[e]={get(){let n=Jt(this,nr(t.open,t.close,this[He]),this[vt]);return Object.defineProperty(this,e,{value:n}),n}};qe.visible={get(){let e=Jt(this,this[He],!0);return Object.defineProperty(this,"visible",{value:e}),e}};var tr=(e,t,n,...r)=>e==="rgb"?t==="ansi16m"?be[n].ansi16m(...r):t==="ansi256"?be[n].ansi256(be.rgbToAnsi256(...r)):be[n].ansi(be.rgbToAnsi(...r)):e==="hex"?tr("rgb",t,n,...be.hexToRgb(...r)):be[n][e](...r),Au=["rgb","hex","ansi256"];for(let e of Au){qe[e]={get(){let{level:n}=this;return function(...r){let o=nr(tr(e,ca[n],"color",...r),be.color.close,this[He]);return Jt(this,o,this[vt])}}};let t="bg"+e[0].toUpperCase()+e.slice(1);qe[t]={get(){let{level:n}=this;return function(...r){let o=nr(tr(e,ca[n],"bgColor",...r),be.bgColor.close,this[He]);return Jt(this,o,this[vt])}}}}var Iu=Object.defineProperties(()=>{},{...qe,level:{enumerable:!0,get(){return this[er].level},set(e){this[er].level=e}}}),nr=(e,t,n)=>{let r,o;return n===void 0?(r=e,o=t):(r=n.openAll+e,o=t+n.closeAll),{open:e,close:t,openAll:r,closeAll:o,parent:n}},Jt=(e,t,n)=>{let r=(...o)=>Ou(r,o.length===1?""+o[0]:o.join(" "));return Object.setPrototypeOf(r,Iu),r[er]=e,r[He]=t,r[vt]=n,r},Ou=(e,t)=>{if(e.level<=0||!t)return e[vt]?"":t;let n=e[He];if(n===void 0)return t;let{openAll:r,closeAll:o}=n;if(t.includes("\x1B"))for(;n!==void 0;)t=oa(t,n.close,n.open),n=n.parent;let i=t.indexOf(`
`);return i!==-1&&(t=ia(t,o,r,i)),r+t+o};Object.defineProperties(kt.prototype,qe);var Nu=kt(),eh=kt({level:aa?aa.level:0});var X=Nu;var M={NETWORK_ERROR:"network_error",RATE_LIMIT:"rate_limit",SERVER_ERROR:"server_error",NO_TOOL_CALL:"no_tool_call",INVALID_TOOL_ARGS:"invalid_tool_args",TOOL_EXECUTION_ERROR:"tool_execution_error",INVALID_RESPONSE:"invalid_response",INVALID_SCHEMA:"invalid_schema",UNKNOWN:"unknown",CONFIG_ERROR:"config_error",AUTH_ERROR:"auth_error",CONTEXT_LENGTH:"context_length",CONTENT_FILTER:"content_filter"},Zu=[M.NETWORK_ERROR,M.RATE_LIMIT,M.SERVER_ERROR,M.NO_TOOL_CALL,M.INVALID_TOOL_ARGS,M.TOOL_EXECUTION_ERROR,M.INVALID_RESPONSE,M.INVALID_SCHEMA,M.UNKNOWN],q=class extends Error{type;retryable;statusCode;rawError;rawResponse;constructor(e,t,n,r){super(t),this.name="InvokeError",this.type=e,this.retryable=Zu.includes(e),this.rawError=n,this.rawResponse=r}},K=console.debug.bind(console,X.gray("[LLM]"));function Ru(e,t){return{type:"function",function:{name:e,description:t.description,parameters:Yn(t.inputSchema,{target:"openapi-3.0"})}}}function Lu(e,t){let n=e.model||"";if(!n)return e;let r=Fu(t),o=Mu(n);if(o.startsWith("qwen")&&(K("Patch Qwen: disable thinking"),e.enable_thinking=!1,e.temperature===void 0&&!/max|plus/.test(o)&&(K("Patch Qwen: raise temperature to 1.0"),e.temperature=1)),o.startsWith("deepseek")&&(K("Patch DeepSeek: disable thinking, remove tool_choice"),e.thinking={type:"disabled"},delete e.tool_choice),o.startsWith("gpt")&&(o.startsWith("gpt-5")&&(e.verbosity="low",e.reasoning_effort=/^gpt-5(-|$)/.test(o)?"minimal":"none",K(`Patch GPT-5: verbosity=low, reasoning_effort=${e.reasoning_effort}`)),o.includes("chat-latest")&&(K("Omitting reasoning_effort and temperature for chat-latest"),delete e.reasoning_effort,delete e.temperature)),o.startsWith("claude")&&(/opus|sonnet|haiku/.test(o)?(K("Patch Claude: disable thinking"),e.thinking={type:"disabled"},r!=="openrouter"&&(e.tool_choice==="required"?(K('Applying Claude patch: convert tool_choice "required" to { type: "any" }'),e.tool_choice={type:"any"}):e.tool_choice?.function?.name&&(K("Applying Claude patch: convert tool_choice format"),e.tool_choice={type:"tool",name:e.tool_choice.function.name}))):(K("Patch Claude: reasoning_effort=low"),e.reasoning_effort="low",delete e.tool_choice)),o.startsWith("gemini")&&(K("Patch Gemini: reasoning_effort=low"),e.reasoning_effort="low",/^gemini-25(?!.*pro)/.test(o)?(K("Patch Gemini 2.5 non-Pro: reasoning_effort=none"),e.reasoning_effort="none"):(o.startsWith("gemini-35-flash")||o.startsWith("gemini-31-flash-lite")||o.startsWith("gemini-3-flash"))&&(K("Patch Gemini 3.x Flash/Lite: reasoning_effort=minimal"),e.reasoning_effort="minimal")),o.startsWith("glm")&&(K("Patch GLM: disable thinking"),e.thinking={type:"disabled"}),o.startsWith("grok")&&(/^grok-4-?3/.test(o)?(K("Patch Grok 4.3: reasoning_effort=none"),e.reasoning_effort="none"):(o.startsWith("grok-3-mini")||o.startsWith("grok-code-fast"))&&(K("Patch Grok mini/code: reasoning_effort=low"),e.reasoning_effort="low")),o.startsWith("kimi")&&(o.includes("code")||(K("Patch Kimi: disable thinking"),e.thinking={type:"disabled"})),o.startsWith("minimax")&&(K("Patch MiniMax: remove parallel_tool_calls"),delete e.parallel_tool_calls,o.includes("m3")&&(K("Patch MiniMax: disable thinking"),e.thinking={type:"disabled"})),r==="openrouter"){let i=e.reasoning_effort;e.thinking?.type==="disabled"||e.enable_thinking===!1||i==="none"?e.reasoning={enabled:!1}:i&&(e.reasoning={enabled:!0,effort:i})}return e}function Mu(e){let t=e.toLowerCase();return t.includes("/")&&(t=t.split("/")[1]),t=t.replace(/_/g,""),t=t.replace(/\./g,""),t}function Fu(e){if(e)try{return new URL(e).hostname==="openrouter.ai"?"openrouter":void 0}catch{return}}var Du=class{config;fetch;constructor(e){this.config=e,this.fetch=e.customFetch}async invoke(e,t,n,r){n?.throwIfAborted();let o=Object.entries(t).map(([b,k])=>Ru(b,k)),i="required";r?.toolChoiceName&&!this.config.disableNamedToolChoice&&(i={type:"function",function:{name:r.toolChoiceName}});let s={model:this.config.model,messages:e,tools:o,parallel_tool_calls:!1,tool_choice:i};this.config.temperature!==void 0&&(s.temperature=this.config.temperature),Lu(s,this.config.baseURL);let a;try{a=this.config.transformRequestBody(s)}catch(b){throw new q(M.CONFIG_ERROR,`transformRequestBody failed: ${b.message}`,b)}let u=a??s,l;try{l=await this.fetch(`${this.config.baseURL}/chat/completions`,{method:"POST",headers:{"Content-Type":"application/json",...this.config.apiKey&&{Authorization:`Bearer ${this.config.apiKey}`}},body:JSON.stringify(u),signal:n})}catch(b){throw b?.name==="AbortError"?b:(console.error(b),new q(M.NETWORK_ERROR,"Network request failed",b))}if(!l.ok){let b;try{b=await l.json()}catch(W){if(W?.name==="AbortError")throw W}let k=b?.error?.message||l.statusText;throw l.status===401||l.status===403?new q(M.AUTH_ERROR,`Authentication failed: ${k}`,b):l.status===429?new q(M.RATE_LIMIT,`Rate limit exceeded: ${k}`,b):l.status>=500?new q(M.SERVER_ERROR,`Server error: ${k}`,b):new q(M.UNKNOWN,`HTTP ${l.status}: ${k}`,b)}let p;try{p=await l.json()}catch(b){throw b?.name==="AbortError"?b:new q(M.INVALID_RESPONSE,"Response body is not valid JSON",b)}let m=p.choices?.[0];if(!m)throw new q(M.INVALID_SCHEMA,"No choices in response",p);switch(m.finish_reason){case"tool_calls":case"function_call":case"stop":break;case"length":throw new q(M.CONTEXT_LENGTH,"Response truncated: max tokens reached",void 0,p);case"content_filter":throw new q(M.CONTENT_FILTER,"Content filtered by safety system",void 0,p);default:throw new q(M.INVALID_SCHEMA,`Unexpected finish_reason: ${m.finish_reason}`,void 0,p)}let d=(r?.normalizeResponse?r.normalizeResponse(p):p).choices?.[0],f=d?.message?.tool_calls?.[0]?.function?.name;if(!f)throw new q(M.NO_TOOL_CALL,"No tool call found in response",void 0,p);let g=t[f];if(!g)throw new q(M.UNKNOWN,`Tool "${f}" not found in tools`,void 0,p);let O=d.message?.tool_calls?.[0]?.function?.arguments;if(!O)throw new q(M.INVALID_TOOL_ARGS,"No tool call arguments found",void 0,p);let w;try{w=JSON.parse(O)}catch(b){throw new q(M.INVALID_TOOL_ARGS,"Failed to parse tool arguments as JSON",b,p)}let C=g.inputSchema.safeParse(w);if(!C.success)throw console.error(ut(C.error)),new q(M.INVALID_TOOL_ARGS,"Tool arguments validation failed",C.error,p);let P=C.data,A;try{A=await g.execute(P)}catch(b){throw b?.name==="AbortError"?b:new q(M.TOOL_EXECUTION_ERROR,`Tool execution failed: ${b?.message}`,b,p)}return{toolCall:{name:f,args:P},toolResult:A,usage:{promptTokens:p.usage?.prompt_tokens??0,completionTokens:p.usage?.completion_tokens??0,totalTokens:p.usage?.total_tokens??0,cachedTokens:p.usage?.prompt_tokens_details?.cached_tokens,reasoningTokens:p.usage?.completion_tokens_details?.reasoning_tokens},rawResponse:p,rawRequest:u}}},ua=class extends EventTarget{config;client;constructor(e){super(),this.config=Uu(e),this.client=new Du(this.config)}async invoke(e,t,n,r){return await ju(async()=>this.client.invoke(e,t,n,r),{maxRetries:this.config.maxRetries,onRetry:(o,i)=>{this.dispatchEvent(new CustomEvent("retry",{detail:{attempt:o,maxAttempts:this.config.maxRetries,lastError:i}}))}})}};async function ju(e,t){let n=0;for(;;)try{return await e()}catch(r){if(r?.name==="AbortError"||r instanceof q&&!r.retryable||(n++,n>t.maxRetries))throw r;console.debug("[LLM] retryable failure, will retry:",r),t.onRetry(n,r),await new Promise(o=>setTimeout(o,100))}}function Uu(e){if(!e.baseURL||!e.model)throw new Error("[PageAgent] LLM configuration required. Please provide: baseURL, model. See: https://alibaba.github.io/page-agent/docs/features/models");return e.temperature!==void 0&&console.warn("[PageAgent] LLMConfig.temperature is deprecated and will be removed in a future version. Use transformRequestBody to set it only for models you have verified accept it."),{baseURL:e.baseURL,model:e.model,apiKey:e.apiKey||"",temperature:e.temperature,maxRetries:e.maxRetries??2,transformRequestBody:e.transformRequestBody??(t=>t),disableNamedToolChoice:e.disableNamedToolChoice??!1,customFetch:(e.customFetch??fetch).bind(globalThis)}}var Bu=`You are an AI agent designed to operate in an iterative loop to automate browser tasks. Your ultimate goal is accomplishing the task provided in <user_request>.

<intro>
You excel at following tasks:
1. Navigating complex websites and extracting precise information
2. Automating form submissions and interactive web actions
3. Gathering and saving information 
4. Operate effectively in an agent loop
5. Efficiently performing diverse web tasks
</intro>

<language_settings>
- Default working language: **English**
- Use the language that user is using. Return in user's language.
</language_settings>

<input>
At every step, your input will consist of: 
1. <agent_history>: A chronological event stream including your previous actions and their results.
2. <agent_state>: Current <user_request> and <step_info>.
3. <browser_state>: Current URL, interactive elements indexed for actions, and visible page content.
</input>

<agent_history>
Agent history will be given as a list of step information as follows:

<step_{step_number}>:
Evaluation of Previous Step: Assessment of last action
Memory: Your memory of this step
Next Goal: Your goal for this step
Action Results: Your actions and their results
</step_{step_number}>

and system messages wrapped in <sys> tag.
</agent_history>

<user_request>
USER REQUEST: This is your ultimate objective and always remains visible.
- This has the highest priority. Make the user happy.
- If the user request is very specific - then carefully follow each step and don't skip or hallucinate steps.
- If the task is open ended you can plan yourself how to get it done.
</user_request>

<browser_state>
1. Browser State will be given as:

Current URL: URL of the page you are currently viewing.
Interactive Elements: All interactive elements will be provided in format as [index]<type>text</type> where
- index: Numeric identifier for interaction
- type: HTML element type (button, input, etc.)
- text: Element description

Examples:
[33]<div>User form</div>
\\t*[35]<button aria-label='Submit form'>Submit</button>

Note that:
- Only elements with numeric indexes in [] are interactive
- (stacked) indentation (with \\t) is important and means that the element is a (html) child of the element above (with a lower index)
- Elements tagged with \`*[\` are the new clickable elements that appeared on the website since the last step - if url has not changed.
- Pure text elements without [] are not interactive.
</browser_state>

<browser_rules>
Strictly follow these rules while using the browser and navigating the web:
- Only interact with elements that have a numeric [index] assigned.
- Only use indexes that are explicitly provided.
- If the page changes after, for example, an input text action, analyze if you need to interact with new elements, e.g. selecting the right option from the list.
- By default, only elements in the visible viewport are listed. Use scrolling actions if you suspect relevant content is offscreen which you need to interact with. Scroll ONLY if there are more pixels below or above the page.
- You can scroll by a specific number of pages using the num_pages parameter (e.g., 0.5 for half page, 2.0 for two pages).
- All the elements that are scrollable are marked with \`data-scrollable\` attribute. Including the scrollable distance in every directions. You can scroll *the element* in case some area are overflowed.
- If a captcha appears, tell user you can not solve captcha. Finish the task and ask user to solve it.
- If the page is not fully loaded, use the \`wait\` action.
- Do not repeat one action for more than 3 times unless some conditions changed.
- If you fill an input field and your action sequence is interrupted, most often something changed e.g. suggestions popped up under the field.
- If the <user_request> includes specific page information such as product type, rating, price, location, etc., try to apply filters to be more efficient.
- The <user_request> is the ultimate goal. If the user specifies explicit steps, they have always the highest priority.
- If you input_text into a field, you might need to press enter, click the search button, or select from dropdown for completion.
- Don't login into a page if you don't have to. Don't login if you don't have the credentials. 
- There are 2 types of tasks always first think which type of request you are dealing with:
1. Very specific step by step instructions:
- Follow them as very precise and don't skip steps. Try to complete everything as requested.
2. Open ended tasks. Plan yourself, be creative in achieving them.
- If you get stuck e.g. with logins or captcha in open-ended tasks you can re-evaluate the task and try alternative ways, e.g. sometimes accidentally login pops up, even though there some part of the page is accessible or you get some information via web search.
</browser_rules>

<capability>
- You can only handle single page app. Do not jump out of current page.
- Do not click on link if it will open in a new page (e.g., <a target="_blank">)
- It is ok to fail the task.
	- User can be wrong. If the request of user is not achievable, inappropriate or you do not have enough information or tools to achieve it. Tell user to make a better request.
	- Webpage can be broken. All webpages or apps have bugs. Some bug will make it hard for your job. It's encouraged to tell user the problem of current page. Your feedbacks (including failing) are valuable for user.
	- Trying too hard can be harmful. Repeating some action back and forth or pushing for a complex procedure with little knowledge can cause unwanted results and harmful side-effects. User would rather you complete the task with a fail.
- If you do not have knowledge for the current webpage or task. You must require user to give specific instructions and detailed steps.
</capability>

<task_completion_rules>
You must call the \`done\` action in one of three cases:
- When you have fully completed the USER REQUEST.
- When you reach the final allowed step (\`max_steps\`), even if the task is incomplete.
- When you feel stuck or unable to solve user request. Or user request is not clear or contains inappropriate content.
- If it is ABSOLUTELY IMPOSSIBLE to continue.

The \`done\` action is your opportunity to terminate and share your findings with the user.
- Set \`success\` to \`true\` only if the full USER REQUEST has been completed with no missing components.
- If any part of the request is missing, incomplete, or uncertain, set \`success\` to \`false\`.
- You can use the \`text\` field of the \`done\` action to communicate your findings and to provide a coherent reply to the user and fulfill the USER REQUEST.
- You are ONLY ALLOWED to call \`done\` as a single action. Don't call it together with other actions.
- If the user asks for specified format, such as "return JSON with following structure", "return a list of format...", MAKE sure to use the right format in your answer.
- If the user asks for a structured output, your \`done\` action's schema may be modified. Take this schema into account when solving the task!
</task_completion_rules>

<reasoning_rules>
Exhibit the following reasoning patterns to successfully achieve the <user_request>:

- Reason about <agent_history> to track progress and context toward <user_request>.
- Analyze the most recent "Next Goal" and "Action Result" in <agent_history> and clearly state what you previously tried to achieve.
- Analyze all relevant items in <agent_history> and <browser_state> to understand your state.
- Explicitly judge success/failure/uncertainty of the last action. Never assume an action succeeded just because it appears to be executed in your last step in <agent_history>. If the expected change is missing, mark the last action as failed (or uncertain) and plan a recovery.
- Analyze whether you are stuck, e.g. when you repeat the same actions multiple times without any progress. Then consider alternative approaches e.g. scrolling for more context or ask user for help.
- Ask user for help if you have any difficulty. Keep user in the loop.
- If you see information relevant to <user_request>, plan saving the information to memory.
- Always reason about the <user_request>. Make sure to carefully analyze the specific steps and information required. E.g. specific filters, specific form fields, specific information to search. Make sure to always compare the current trajectory with the user request and think carefully if thats how the user requested it.
</reasoning_rules>

<examples>
Here are examples of good output patterns. Use them as reference but never copy them directly.

<evaluation_examples>
"evaluation_previous_goal": "Successfully navigated to the product page and found the target information. Verdict: Success"
"evaluation_previous_goal": "Clicked the login button and user authentication form appeared. Verdict: Success"
</evaluation_examples>

<memory_examples>
"memory": "Found many pending reports that need to be analyzed in the main page. Successfully processed the first 2 reports on quarterly sales data and moving on to inventory analysis and customer feedback reports."
</memory_examples>

<next_goal_examples>
"next_goal": "Click on the 'Add to Cart' button to proceed with the purchase flow."
</next_goal_examples>
</examples>

<output>
{
  "evaluation_previous_goal": "Concise one-sentence analysis of your last action. Clearly state success, failure, or uncertain.",
  "memory": "1-3 concise sentences of specific memory of this step and overall progress. You should put here everything that will help you track progress in future steps. Like counting pages visited, items found, etc.",
  "next_goal": "State the next immediate goal and action to achieve it, in one clear sentence.",
  "action":{
    "Action name": {// Action parameters}
  }
}
</output>
`,Ye=console.log.bind(console,X.yellow("[autoFixer]"));function Wu(e,t){let n,r=e.choices?.[0];if(!r)throw new Error("No choices in response");let o=r.message;if(!o)throw new Error("No message in choice");let i=o.tool_calls?.[0];if(i?.function?.arguments)n=Ie(i.function.arguments),i.function.name&&i.function.name!=="AgentOutput"&&(Ye("#1: fixing tool_call"),n={action:Ie(n)});else if(o.content){let s=Gu(o.content.trim());if(s)n=Ie(s),n?.name==="AgentOutput"&&(Ye("#2: fixing tool_call"),n=Ie(n.arguments)),n?.type==="function"&&(Ye("#3: fixing tool_call"),n=Ie(n.function.arguments)),!n?.action&&!n?.evaluation_previous_goal&&!n?.memory&&!n?.next_goal&&!n?.thinking&&(Ye("#4: fixing tool_call"),n={action:Ie(n)});else throw new Error("No tool_call and the message content does not contain valid JSON")}else throw new Error("No tool_call nor message content is present");return n=Ie(n),n.action&&(n.action=Ie(n.action)),n.action&&t&&(n.action=Vu(n.action,t)),n.action||(Ye("#5: fixing tool_call"),n.action={wait:{seconds:1}}),{...e,choices:[{...r,message:{...o,tool_calls:[{...i||{},function:{...i?.function||{},name:"AgentOutput",arguments:JSON.stringify(n)}}]}}]}}function Vu(e,t){if(typeof e!="object"||e===null)return e;let n=Object.keys(e)[0];if(!n)return e;let r=t.get(n);if(!r){let a=Array.from(t.keys()).join(", ");throw new q(M.INVALID_TOOL_ARGS,`Unknown action "${n}". Available: ${a}`)}let o=e[n],i=r.inputSchema;if(i instanceof Kn&&o!==null&&typeof o!="object"){let a=Object.keys(i.shape).find(u=>!i.shape[u].safeParse(void 0).success);a&&(Ye(`coercing primitive action input for "${n}"`),o={[a]:o})}let s=i.safeParse(o);if(!s.success)throw new q(M.INVALID_TOOL_ARGS,`Invalid input for action "${n}": ${ut(s.error)}`);return{[n]:s.data}}function Ie(e){if(typeof e=="string")try{return JSON.parse(e.trim())}catch{return e}return e}function Gu(e){try{let t=/({[\s\S]*})/.exec(e)??[];return t.length===0?null:JSON.parse(t[0])}catch{return null}}async function ir(e,t){if(!t){await new Promise(n=>setTimeout(n,e*1e3));return}t.throwIfAborted(),await new Promise((n,r)=>{let o=setTimeout(()=>{t.removeEventListener("abort",i),n()},e*1e3),i=()=>{clearTimeout(o),r(t.reason)};t.addEventListener("abort",i,{once:!0})})}function Ju(e,t){return e.length>t?e.substring(0,t)+"...":e}function Hu(e){let t=Math.random().toString(36).substring(2,11);if(!e)return t;let n=1e3,r=0;for(;e.includes(t);)if(t=Math.random().toString(36).substring(2,11),r++,r>n)throw new Error("randomID: too many tries");return t}var sr=globalThis;sr.__PAGE_AGENT_IDS__||(sr.__PAGE_AGENT_IDS__=[]);var la=sr.__PAGE_AGENT_IDS__;function pa(){let e=Hu(la);return la.push(e),e}var rr=new Map;async function qu(e){let t;try{t=new URL(e).origin}catch{return null}if(t==="null")return null;if(rr.has(t))return rr.get(t);let n=`${t}/llms.txt`,r=null;try{console.log(X.gray(`[llms.txt] Fetching ${n}`));let o=await fetch(n,{signal:AbortSignal.timeout(3e3)});o.ok?(r=await o.text(),console.log(X.green(`[llms.txt] Found (${r.length} chars)`)),r.length>1e3&&(console.log(X.yellow("[llms.txt] Truncating to 1000 chars")),r=Ju(r,1e3))):console.debug(X.gray(`[llms.txt] ${o.status} for ${n}`))}catch(o){console.debug(X.gray(`[llms.txt] not found for ${n}`),o)}return rr.set(t,r),r}function Yu(e,t,n){if(!e){let r=t??"Assertion failed";throw n||console.error(X.red(`\u274C assert: ${r}`)),new Error(r)}}async function or(e){try{return await e()}catch(t){console.error(t);return}}function uh(e){return e}var we=new Map;we.set("done",{description:"Complete task. Text is your final response to the user \u2014 keep it concise unless the user explicitly asks for detail.",inputSchema:de({text:$e(),success:Vt().default(!0)}),execute:async function(e){return Promise.resolve("Task completed")}});we.set("wait",{description:"Wait for x seconds. Can be used to wait until the page or data is fully loaded.",inputSchema:de({seconds:Re().min(1).max(10).default(1)}),execute:async function(e,{signal:t}){let n=await this.pageController.getLastUpdateTime(),r=(Date.now()-n)/1e3,o=Math.max(0,e.seconds-r);return console.log(`actualWaitTime: ${o} seconds`),await ir(o,t),`\u2705 Waited for ${(r+o).toFixed(2)} seconds.`}});we.set("ask_user",{description:"Ask the user a question and wait for their answer. Use this if you need more information or clarification.",inputSchema:de({question:$e()}),execute:async function(e,{signal:t}){if(!this.onAskUser)throw new Error("ask_user tool requires onAskUser callback to be set");return`User answered: ${await this.onAskUser(e.question,{signal:t})}`}});we.set("click_element_by_index",{description:"Click element by index",inputSchema:de({index:Je().min(0)}),execute:async function(e){return(await this.pageController.clickElement(e.index)).message}});we.set("input_text",{description:"Click and type text into an interactive input element",inputSchema:de({index:Je().min(0),text:$e()}),execute:async function(e){return(await this.pageController.inputText(e.index,e.text)).message}});we.set("select_dropdown_option",{description:"Select dropdown option for interactive element index by the text of the option you want to select",inputSchema:de({index:Je().min(0),text:$e()}),execute:async function(e){return(await this.pageController.selectOption(e.index,e.text)).message}});we.set("scroll",{description:"Scroll vertically. Without index: scrolls the document. With index: scrolls the container at that index (or its nearest scrollable ancestor). Use index of a data-scrollable element to scroll a specific area.",inputSchema:de({down:Vt().default(!0),num_pages:Re().min(0).max(10).optional().default(.1),pixels:Re().int().min(0).optional(),index:Re().int().min(0).optional()}),execute:async function(e){return(await this.pageController.scroll({...e,numPages:e.num_pages})).message}});we.set("scroll_horizontally",{description:"Scroll horizontally. Without index: scrolls the document. With index: scrolls the container at that index (or its nearest scrollable ancestor). Use index of a data-scrollable element to scroll a specific area.",inputSchema:de({right:Vt().default(!0),pixels:Re().int().min(0),index:Re().int().min(0).optional()}),execute:async function(e){return(await this.pageController.scrollHorizontally(e)).message}});we.set("execute_javascript",{description:"Execute JavaScript code on the current page. Supports async/await syntax. Use with caution! An `AbortSignal` named `signal` is available in scope: long-running async code MUST honor it (e.g. `await fetch(url, { signal })`, or `signal.throwIfAborted()` in loops)",inputSchema:de({script:$e()}),execute:async function(e,{signal:t}){let n=await this.pageController.executeJavascript(e.script,t);return t.throwIfAborted(),n.message}});var da=class extends EventTarget{id=pa();config;tools;pageController;task="";taskId="";history=[];disposed=!1;onAskUser;#n="idle";#o;#t=new AbortController;#i=[];#c=Promise.resolve();#a=null;#s={totalWaitTime:0,lastURL:"",browserState:null};constructor(e){if(super(),this.config={...e,maxSteps:e.maxSteps??40},this.#o=new ua(this.config),this.tools=new Map(we),this.pageController=e.pageController,this.#o.addEventListener("retry",t=>{let{attempt:n,maxAttempts:r,lastError:o}=t.detail;this.#p({type:"retrying",attempt:n,maxAttempts:r}),this.history.push({type:"error",message:String(o),rawResponse:o.rawResponse}),this.history.push({type:"retry",message:`LLM retry attempt ${n} of ${r}`,attempt:n,maxAttempts:r}),this.#e()}),this.config.customTools)for(let[t,n]of Object.entries(this.config.customTools)){if(n===null){this.tools.delete(t);continue}this.tools.set(t,n)}this.config.experimentalScriptExecutionTool||this.tools.delete("execute_javascript")}get status(){return this.#n}get lastResult(){return this.#a}#l(){this.dispatchEvent(new Event("statuschange"))}#e(e){e&&this.history.push(e),this.dispatchEvent(new Event("historychange"))}#p(e){this.dispatchEvent(new CustomEvent("activity",{detail:e}))}#h(e){this.#n!==e&&(this.#n=e,this.#l())}pushObservation(e){this.#i.push(e)}async stop(){this.#n==="running"&&(this.#t.abort(),await this.#c)}async execute(e){if(this.disposed)throw new Error("PageAgent has been disposed. Create a new instance.");if(this.#n==="running")throw new Error("A task is already running.");if(!e)throw new Error("Task is required");this.task=e,this.taskId=pa(),this.history=[],this.#i=[],this.#s={totalWaitTime:0,lastURL:"",browserState:null},this.#t=new AbortController;let t=this.#t.signal,n;this.#c=new Promise(d=>n=d),this.#h("running"),this.#e(),this.onAskUser||this.tools.delete("ask_user");let r=this.config.onBeforeStep,o=this.config.onAfterStep,i=this.config.onBeforeTask,s=this.config.onAfterTask,a=this.config.stepDelay??.4,u=this.config.maxSteps,l=0,p,m="error";await or(()=>this.pageController.showMask());try{for(await i?.(this);;){await r?.(this,l);try{console.group(`step: ${l}`),l>0&&await ir(a,t),t.throwIfAborted(),console.log(X.blue.bold("\u{1F440} Observing...")),this.#s.browserState=await this.pageController.getBrowserState(),await this.#m(l);let d=[{role:"system",content:this.#f()},{role:"user",content:await this.#u()}],f={AgentOutput:this.#r()};console.log(X.blue.bold("\u{1F9E0} Thinking...")),this.#p({type:"thinking"});let g=await this.#o.invoke(d,f,t,{toolChoiceName:"AgentOutput",normalizeResponse:k=>Wu(k,this.tools)}),O=g.toolResult,w=O.input,C=O.output,P={evaluation_previous_goal:w.evaluation_previous_goal,memory:w.memory,next_goal:w.next_goal},A=Object.keys(w.action)[0],b={name:A,input:w.action[A],output:C};if(this.#e({type:"step",stepIndex:l,reflection:P,action:b,usage:g.usage,rawResponse:g.rawResponse,rawRequest:g.rawRequest}),A==="done"){let k=b.input?.success??!1,W=b.input?.text||"no text provided";console.log(X.green.bold("Task completed"),k,W),p={success:k,data:W,history:this.history},this.#a=p,m="completed";break}}catch(d){let f=d?.name==="AbortError";f||console.error("Task failed",d);let g=f?"Task aborted":String(d);this.#p({type:"error",message:g}),this.#e({type:"error",message:g,rawResponse:d}),p={success:!1,data:g,history:this.history},this.#a=p,m=f?"stopped":"error";break}finally{console.groupEnd(),await o?.(this,this.history)}if(l++,l>u){let d="Step count exceeded maximum limit";console.error(d),this.#p({type:"error",message:d}),this.#e({type:"error",message:d}),p={success:!1,data:d,history:this.history},this.#a=p,m="error";break}}return await s?.(this,p),p}catch(d){throw this.#p({type:"error",message:String(d)}),m="error",d}finally{await or(()=>this.pageController.cleanUpHighlights()),await or(()=>this.pageController.hideMask()),this.#t.abort(),n(),this.#h(m)}}#r(){let e=this.tools,t=Array.from(e.entries()).map(([r,o])=>de({[r]:o.inputSchema}).describe(o.description)),n=Qn(t);return{description:"You MUST call this tool every step!",inputSchema:de({evaluation_previous_goal:$e().optional(),memory:$e().optional(),next_goal:$e().optional(),action:n}),execute:async r=>{let o=this.#t.signal;o.throwIfAborted(),console.log(X.blue.bold("MacroTool input"),r);let i=r.action,s=Object.keys(i)[0],a=i[s],u=[];r.evaluation_previous_goal&&u.push(`\u2705: ${r.evaluation_previous_goal}`),r.memory&&u.push(`\u{1F4BE}: ${r.memory}`),r.next_goal&&u.push(`\u{1F3AF}: ${r.next_goal}`);let l=u.length>0?u.join(`
`):"";l&&console.log(l);let p=e.get(s);Yu(p,`Tool ${s} not found`),console.log(X.blue.bold(`Executing tool: ${s}`),a),this.#p({type:"executing",tool:s,input:a});let m=Date.now(),d=await p.execute.bind(this)(a,{signal:o});o.throwIfAborted();let f=Date.now()-m;return console.log(X.green.bold(`Tool (${s}) executed for ${f}ms`),d),this.#p({type:"executed",tool:s,input:a,output:d,duration:f}),s==="wait"?this.#s.totalWaitTime+=a?.seconds||0:this.#s.totalWaitTime=0,{input:r,output:d}}}}#f(){if(this.config.customSystemPrompt)return this.config.customSystemPrompt;let e=this.config.language==="zh-CN"?"\u4E2D\u6587":"English";return Bu.replace(/Default working language: \*\*.*?\*\*/,`Default working language: **${e}**`)}async#d(){let{instructions:e,experimentalLlmsTxt:t}=this.config,n=e?.system?.trim(),r,o=this.#s.browserState?.url||"";if(e?.getPageInstructions&&o)try{r=e.getPageInstructions(o)?.trim()}catch(a){console.error(X.red("[PageAgent] Failed to execute getPageInstructions callback:"),a)}let i=t&&o?await qu(o):void 0;if(!n&&!r&&!i)return"";let s=`<instructions>
`;return n&&(s+=`<system_instructions>
${n}
</system_instructions>
`),r&&(s+=`<page_instructions>
${r}
</page_instructions>
`),i&&(s+=`<llms_txt>
${i}
</llms_txt>
`),s+=`</instructions>

`,s}async#m(e){this.#s.totalWaitTime>=3&&this.pushObservation(`You have waited ${this.#s.totalWaitTime} seconds accumulatively. DO NOT wait any longer unless you have a good reason.`);let t=this.#s.browserState?.url||"";t!==this.#s.lastURL&&(this.pushObservation(`Page navigated to \u2192 ${t}`),this.#s.lastURL=t,await ir(.5));let n=this.config.maxSteps-e;if(n===5?this.pushObservation(`\u26A0\uFE0F Only ${n} steps remaining. Consider wrapping up or calling done with partial results.`):n===2&&this.pushObservation(`\u26A0\uFE0F Critical: Only ${n} steps left! You must finish the task or call done immediately.`),this.#i.length>0){for(let r of this.#i)this.history.push({type:"observation",content:r}),console.log(X.cyan("Observation:"),r);this.#i=[],this.#e()}}async#u(){let e=this.#s.browserState,t="";t+=await this.#d();let n=this.history.filter(i=>i.type==="step").length;t+=`<agent_state>
`,t+=`<user_request>
`,t+=`${this.task}
`,t+=`</user_request>
`,t+=`<step_info>
`,t+=`Step ${n+1} of ${this.config.maxSteps} max possible steps
`,t+=`Current time: ${new Date().toLocaleString()}
`,t+=`</step_info>
`,t+=`</agent_state>

`,t+=`<agent_history>
`;let r=0;for(let i of this.history)i.type==="step"?(r++,t+=`<step_${r}>
`,t+=`Evaluation of Previous Step: ${i.reflection.evaluation_previous_goal}
`,t+=`Memory: ${i.reflection.memory}
`,t+=`Next Goal: ${i.reflection.next_goal}
`,t+=`Action Results: ${i.action.output}
`,t+=`</step_${r}>
`):i.type==="observation"?t+=`<sys>${i.content}</sys>
`:i.type==="user_takeover"?t+=`<sys>User took over control and made changes to the page</sys>
`:i.type;t+=`</agent_history>

`;let o=e.content;return this.config.transformPageContent&&(o=await this.config.transformPageContent(o)),t+=`<browser_state>
`,t+=e.header+`
`,t+=o+`
`,t+=e.footer+`

`,t+=`</browser_state>

`,t}dispose(){console.log("Disposing PageAgent..."),this.disposed=!0,this.pageController.dispose(),this.#t.abort(),this.dispatchEvent(new Event("dispose")),this.config.onDispose?.(this)}};var xa=Object.defineProperty,pl=(e,t)=>{let n={};for(var r in e)xa(n,r,{get:e[r],enumerable:!0});return t||xa(n,Symbol.toStringTag,{value:"Module"}),n};function dl(e){return!!e&&e.nodeType===1}function hl(e){return e?.nodeType===1&&e.tagName==="INPUT"}function fl(e){return e?.nodeType===1&&e.tagName==="TEXTAREA"}function ml(e){return e?.nodeType===1&&e.tagName==="SELECT"}function gl(e){return e?.nodeType===1&&e.tagName==="A"}function _l(e){let t=e.ownerDocument.defaultView?.frameElement;if(!t)return{x:0,y:0};let n=t.getBoundingClientRect();return{x:n.left,y:n.top}}function bl(e){return Object.getOwnPropertyDescriptor(Object.getPrototypeOf(e),"value").set}async function Me(e){await new Promise(t=>setTimeout(t,e*1e3))}async function xl(e,t,n){let r=_l(e);window.dispatchEvent(new CustomEvent("PageAgent::MovePointerTo",{detail:{x:t+r.x,y:n+r.y}})),await Me(.3)}async function yl(){window.dispatchEvent(new CustomEvent("PageAgent::ClickPointer"))}async function wl(){window.dispatchEvent(new CustomEvent("PageAgent::EnablePassThrough"))}async function vl(){window.dispatchEvent(new CustomEvent("PageAgent::DisablePassThrough"))}function $t(e,t){let n=e.get(t);if(!n)throw new Error(`No interactive element found at index ${t}`);let r=n.ref;if(!r)throw new Error(`Element at index ${t} does not have a reference`);if(!dl(r))throw new Error(`Element at index ${t} is not an HTMLElement`);return r}var Oe=null;function ka(){Oe&&(Oe.dispatchEvent(new PointerEvent("pointerout",{bubbles:!0})),Oe.dispatchEvent(new PointerEvent("pointerleave",{bubbles:!1})),Oe.dispatchEvent(new MouseEvent("mouseout",{bubbles:!0})),Oe.dispatchEvent(new MouseEvent("mouseleave",{bubbles:!1})),Oe.blur(),Oe=null)}async function $a(e){ka(),Oe=e,await ya(e);let t=e.ownerDocument.defaultView?.frameElement;t&&await ya(t);let n=e.getBoundingClientRect(),r=n.left+n.width/2,o=n.top+n.height/2;await xl(e,r,o),await yl(),await Me(.1);let i=e.ownerDocument;await wl();let s=i.elementFromPoint(r,o);await vl();let a=s instanceof HTMLElement&&e.contains(s)?s:e,u={bubbles:!0,cancelable:!0,clientX:r,clientY:o,pointerType:"mouse"},l={bubbles:!0,cancelable:!0,clientX:r,clientY:o,button:0};a.dispatchEvent(new PointerEvent("pointerover",u)),a.dispatchEvent(new PointerEvent("pointerenter",{...u,bubbles:!1})),a.dispatchEvent(new MouseEvent("mouseover",l)),a.dispatchEvent(new MouseEvent("mouseenter",{...l,bubbles:!1})),a.dispatchEvent(new PointerEvent("pointerdown",u)),a.dispatchEvent(new MouseEvent("mousedown",l)),e.focus({preventScroll:!0}),a.dispatchEvent(new PointerEvent("pointerup",u)),a.dispatchEvent(new MouseEvent("mouseup",l)),a.click(),await Me(.2)}async function kl(e,t){let n=e.isContentEditable;if(!hl(e)&&!fl(e)&&!n)throw new Error("Element is not an input, textarea, or contenteditable");if(await $a(e),n){if(e.dispatchEvent(new InputEvent("beforeinput",{bubbles:!0,cancelable:!0,inputType:"deleteContent"}))&&(e.innerText="",e.dispatchEvent(new InputEvent("input",{bubbles:!0,inputType:"deleteContent"}))),e.dispatchEvent(new InputEvent("beforeinput",{bubbles:!0,cancelable:!0,inputType:"insertText",data:t}))&&(e.innerText=t,e.dispatchEvent(new InputEvent("input",{bubbles:!0,inputType:"insertText",data:t}))),e.innerText.trim()!==t.trim()){e.focus();let r=e.ownerDocument,o=(r.defaultView||window).getSelection(),i=r.createRange();i.selectNodeContents(e),o?.removeAllRanges(),o?.addRange(i),r.execCommand("delete",!1),r.execCommand("insertText",!1,t)}e.dispatchEvent(new Event("change",{bubbles:!0})),e.blur()}else bl(e).call(e,t);n||e.dispatchEvent(new Event("input",{bubbles:!0})),await Me(.1),ka()}async function $l(e,t){if(!ml(e))throw new Error("Element is not a select element");let n=Array.from(e.options).find(r=>r.textContent?.trim()===t.trim());if(!n)throw new Error(`Option with text "${t}" not found in select element`);e.value=n.value,e.dispatchEvent(new Event("change",{bubbles:!0})),await Me(.1)}async function ya(e){let t=e;typeof t.scrollIntoViewIfNeeded=="function"?t.scrollIntoViewIfNeeded():e.scrollIntoView({behavior:"auto",block:"center",inline:"nearest"})}async function zl(e,t){if(t){let s=t,a=s,u=!1,l=null,p=0,m=0,d=e;for(;a&&m<10;){let f=window.getComputedStyle(a),g=/(auto|scroll|overlay)/.test(f.overflowY)||f.scrollbarWidth&&f.scrollbarWidth!=="auto"||f.scrollbarGutter&&f.scrollbarGutter!=="auto",O=a.scrollHeight>a.clientHeight;if(g&&O){let w=a.scrollTop,C=a.scrollHeight-a.clientHeight,P=d/3;P>0?P=Math.min(P,C-w):P=Math.max(P,-w),a.scrollTop=w+P;let A=a.scrollTop-w;if(Math.abs(A)>.5){u=!0,l=a,p=A;break}}if(a===document.body||a===document.documentElement)break;a=a.parentElement,m++}return u?`Scrolled container (${l?.tagName}) by ${p}px`:`No scrollable container found for element (${s.tagName})`}let n=e,r=s=>s.clientHeight>=window.innerHeight*.5,o=s=>!!(s&&/(auto|scroll|overlay)/.test(getComputedStyle(s).overflowY)&&s.scrollHeight>s.clientHeight&&r(s)),i=document.activeElement;for(;i&&!o(i)&&i!==document.body;)i=i.parentElement;if(i=o(i)?i:Array.from(document.querySelectorAll("*")).find(o)||document.scrollingElement||document.documentElement,i===document.scrollingElement||i===document.documentElement||i===document.body){let s=window.scrollY,a=document.documentElement.scrollHeight-window.innerHeight;window.scrollBy(0,n);let u=window.scrollY,l=u-s;if(Math.abs(l)<1)return n>0?"\u26A0\uFE0F Already at the bottom of the page, cannot scroll down further.":"\u26A0\uFE0F Already at the top of the page, cannot scroll up further.";let p=n>0&&u>=a-1,m=n<0&&u<=1;return p?`\u2705 Scrolled page by ${l}px. Reached the bottom of the page.`:m?`\u2705 Scrolled page by ${l}px. Reached the top of the page.`:`\u2705 Scrolled page by ${l}px.`}else{let s="The document is not scrollable. Falling back to container scroll.";console.log(`[PageController] ${s}`);let a=i.scrollTop,u=i.scrollHeight-i.clientHeight;i.scrollBy({top:n,behavior:"smooth"}),await Me(.1);let l=i.scrollTop,p=l-a;if(Math.abs(p)<1)return n>0?`\u26A0\uFE0F ${s} Already at the bottom of container (${i.tagName}), cannot scroll down further.`:`\u26A0\uFE0F ${s} Already at the top of container (${i.tagName}), cannot scroll up further.`;let m=n>0&&l>=u-1,d=n<0&&l<=1;return m?`\u2705 ${s} Scrolled container (${i.tagName}) by ${p}px. Reached the bottom.`:d?`\u2705 ${s} Scrolled container (${i.tagName}) by ${p}px. Reached the top.`:`\u2705 ${s} Scrolled container (${i.tagName}) by ${p}px.`}}async function El(e,t){if(t){let s=t,a=s,u=!1,l=null,p=0,m=0,d=e;for(;a&&m<10;){let f=window.getComputedStyle(a),g=/(auto|scroll|overlay)/.test(f.overflowX)||f.scrollbarWidth&&f.scrollbarWidth!=="auto"||f.scrollbarGutter&&f.scrollbarGutter!=="auto",O=a.scrollWidth>a.clientWidth;if(g&&O){let w=a.scrollLeft,C=a.scrollWidth-a.clientWidth,P=d/3;P>0?P=Math.min(P,C-w):P=Math.max(P,-w),a.scrollLeft=w+P;let A=a.scrollLeft-w;if(Math.abs(A)>.5){u=!0,l=a,p=A;break}}if(a===document.body||a===document.documentElement)break;a=a.parentElement,m++}return u?`Scrolled container (${l?.tagName}) horizontally by ${p}px`:`No horizontally scrollable container found for element (${s.tagName})`}let n=e,r=s=>s.clientWidth>=window.innerWidth*.5,o=s=>!!(s&&/(auto|scroll|overlay)/.test(getComputedStyle(s).overflowX)&&s.scrollWidth>s.clientWidth&&r(s)),i=document.activeElement;for(;i&&!o(i)&&i!==document.body;)i=i.parentElement;if(i=o(i)?i:Array.from(document.querySelectorAll("*")).find(o)||document.scrollingElement||document.documentElement,i===document.scrollingElement||i===document.documentElement||i===document.body){let s=window.scrollX,a=document.documentElement.scrollWidth-window.innerWidth;window.scrollBy(n,0);let u=window.scrollX,l=u-s;if(Math.abs(l)<1)return n>0?"\u26A0\uFE0F Already at the right edge of the page, cannot scroll right further.":"\u26A0\uFE0F Already at the left edge of the page, cannot scroll left further.";let p=n>0&&u>=a-1,m=n<0&&u<=1;return p?`\u2705 Scrolled page by ${l}px. Reached the right edge of the page.`:m?`\u2705 Scrolled page by ${l}px. Reached the left edge of the page.`:`\u2705 Scrolled page horizontally by ${l}px.`}else{let s="The document is not scrollable. Falling back to container scroll.";console.log(`[PageController] ${s}`);let a=i.scrollLeft,u=i.scrollWidth-i.clientWidth;i.scrollBy({left:n,behavior:"smooth"}),await Me(.1);let l=i.scrollLeft,p=l-a;if(Math.abs(p)<1)return n>0?`\u26A0\uFE0F ${s} Already at the right edge of container (${i.tagName}), cannot scroll right further.`:`\u26A0\uFE0F ${s} Already at the left edge of container (${i.tagName}), cannot scroll left further.`;let m=n>0&&l>=u-1,d=n<0&&l<=1;return m?`\u2705 ${s} Scrolled container (${i.tagName}) by ${p}px. Reached the right edge.`:d?`\u2705 ${s} Scrolled container (${i.tagName}) by ${p}px. Reached the left edge.`:`\u2705 ${s} Scrolled container (${i.tagName}) horizontally by ${p}px.`}}var Sl=(e={doHighlightElements:!0,focusHighlightIndex:-1,viewportExpansion:0,debugMode:!1,interactiveBlacklist:[],interactiveWhitelist:[],highlightOpacity:.1,highlightLabelOpacity:.5})=>{let{interactiveBlacklist:t,interactiveWhitelist:n,highlightOpacity:r,highlightLabelOpacity:o}=e,{doHighlightElements:i,focusHighlightIndex:s,viewportExpansion:a,debugMode:u}=e,l=0,p=new WeakMap;function m(c,_){!c||c.nodeType!==Node.ELEMENT_NODE||p.set(c,{...p.get(c),..._})}let d={boundingRects:new WeakMap,clientRects:new WeakMap,computedStyles:new WeakMap,clearCache:()=>{d.boundingRects=new WeakMap,d.clientRects=new WeakMap,d.computedStyles=new WeakMap}};function f(c){if(!c)return null;if(d.boundingRects.has(c))return d.boundingRects.get(c);let _=c.getBoundingClientRect();return _&&d.boundingRects.set(c,_),_}function g(c){if(!c)return null;if(d.computedStyles.has(c))return d.computedStyles.get(c);let _=window.getComputedStyle(c);return _&&d.computedStyles.set(c,_),_}function O(c){if(!c)return null;if(d.clientRects.has(c))return d.clientRects.get(c);let _=c.getClientRects();return _&&d.clientRects.set(c,_),_}let w={},C={current:0},P="playwright-highlight-container";function A(c,_,T=null){if(!c)return _;let x=[],$=null,N=20,y=16,S=null;try{let v=document.getElementById(P);v||(v=document.createElement("div"),v.id=P,v.style.position="fixed",v.style.pointerEvents="none",v.style.top="0",v.style.left="0",v.style.width="100%",v.style.height="100%",v.style.zIndex="2147483640",v.style.backgroundColor="transparent",document.body.appendChild(v));let I=c.getClientRects();if(!I||I.length===0)return _;let ue=["#FF0000","#00FF00","#0000FF","#FFA500","#800080","#008080","#FF69B4","#4B0082","#FF4500","#2E8B57","#DC143C","#4682B4"],Q=ue[_%ue.length],_e=Q+Math.floor(r*255).toString(16).padStart(2,"0");Q=Q+Math.floor(o*255).toString(16).padStart(2,"0");let oe={x:0,y:0};if(T){let G=T.getBoundingClientRect();oe.x=G.left,oe.y=G.top}let tt=document.createDocumentFragment();for(let G of I){if(G.width===0||G.height===0)continue;let J=document.createElement("div");J.style.position="fixed",J.style.border=`2px solid ${Q}`,J.style.backgroundColor=_e,J.style.pointerEvents="none",J.style.boxSizing="border-box";let D=G.top+oe.y,ve=G.left+oe.x;J.style.top=`${D}px`,J.style.left=`${ve}px`,J.style.width=`${G.width}px`,J.style.height=`${G.height}px`,tt.appendChild(J),x.push({element:J,initialRect:G})}let Ne=I[0];$=document.createElement("div"),$.className="playwright-highlight-label",$.style.position="fixed",$.style.background=Q,$.style.color="white",$.style.padding="1px 4px",$.style.borderRadius="4px",$.style.fontSize=`${Math.min(12,Math.max(8,Ne.height/2))}px`,$.textContent=_.toString(),N=$.offsetWidth>0?$.offsetWidth:N,y=$.offsetHeight>0?$.offsetHeight:y;let pr=Ne.top+oe.y,Yt=Ne.left+oe.x,zt=pr+2,Fe=Yt+Ne.width-N-2;(Ne.width<N+4||Ne.height<y+4)&&(zt=pr-y-2,Fe=Yt+Ne.width-N,Fe<oe.x&&(Fe=Yt)),zt=Math.max(0,Math.min(zt,window.innerHeight-y)),Fe=Math.max(0,Math.min(Fe,window.innerWidth-N)),$.style.top=`${zt}px`,$.style.left=`${Fe}px`,tt.appendChild($);let Et=((G,J)=>{let D=0;return(...ve)=>{let ae=performance.now();if(!(ae-D<J))return D=ae,G(...ve)}})(()=>{let G=c.getClientRects(),J={x:0,y:0};if(T){let D=T.getBoundingClientRect();J.x=D.left,J.y=D.top}if(x.forEach((D,ve)=>{if(ve<G.length){let ae=G[ve],De=ae.top+J.y,Ee=ae.left+J.x;D.element.style.top=`${De}px`,D.element.style.left=`${Ee}px`,D.element.style.width=`${ae.width}px`,D.element.style.height=`${ae.height}px`,D.element.style.display=ae.width===0||ae.height===0?"none":"block"}else D.element.style.display="none"}),G.length<x.length)for(let D=G.length;D<x.length;D++)x[D].element.style.display="none";if($&&G.length>0){let D=G[0],ve=D.top+J.y,ae=D.left+J.x,De=ve+2,Ee=ae+D.width-N-2;(D.width<N+4||D.height<y+4)&&(De=ve-y-2,Ee=ae+D.width-N,Ee<J.x&&(Ee=ae)),De=Math.max(0,Math.min(De,window.innerHeight-y)),Ee=Math.max(0,Math.min(Ee,window.innerWidth-N)),$.style.top=`${De}px`,$.style.left=`${Ee}px`,$.style.display="block"}else $&&($.style.display="none")},16);return window.addEventListener("scroll",Et,!0),window.addEventListener("resize",Et),S=()=>{window.removeEventListener("scroll",Et,!0),window.removeEventListener("resize",Et),x.forEach(G=>G.element.remove()),$&&$.remove()},v.appendChild(tt),_+1}finally{S&&(window._highlightCleanupFunctions=window._highlightCleanupFunctions||[]).push(S)}}function b(c){if(!c||c.nodeType!==Node.ELEMENT_NODE)return null;let _=g(c);if(!_)return null;let T=_.display;if(T==="inline"||T==="inline-block")return null;let x=_.overflowX,$=_.overflowY,N=_.scrollbarWidth&&_.scrollbarWidth!=="auto"||_.scrollbarGutter&&_.scrollbarGutter!=="auto",y=x==="auto"||x==="scroll",S=$==="auto"||$==="scroll";if(!y&&!S&&!N)return null;let v=c.scrollWidth-c.clientWidth,I=c.scrollHeight-c.clientHeight,ue=4;if(v<ue&&I<ue||!S&&!N&&v<ue||!y&&!N&&I<ue)return null;let Q=c.scrollTop,_e=c.scrollLeft,oe={top:Q,right:c.scrollWidth-c.clientWidth-c.scrollLeft,bottom:c.scrollHeight-c.clientHeight-c.scrollTop,left:_e};return m(c,{scrollable:!0,scrollData:oe}),oe}function k(c){try{if(a===-1){let y=c.parentElement;if(!y)return!1;try{return y.checkVisibility({checkOpacity:!0,checkVisibilityCSS:!0})}catch{let v=window.getComputedStyle(y);return v.display!=="none"&&v.visibility!=="hidden"&&v.opacity!=="0"}}let _=document.createRange();_.selectNodeContents(c);let T=_.getClientRects();if(!T||T.length===0)return!1;let x=!1,$=!1;for(let y of T)if(y.width>0&&y.height>0&&(x=!0,!(y.bottom<-a||y.top>window.innerHeight+a||y.right<-a||y.left>window.innerWidth+a))){$=!0;break}if(!x||!$)return!1;let N=c.parentElement;if(!N)return!1;try{return N.checkVisibility({checkOpacity:!0,checkVisibilityCSS:!0})}catch{let S=window.getComputedStyle(N);return S.display!=="none"&&S.visibility!=="hidden"&&S.opacity!=="0"}}catch(_){return console.warn("Error checking text node visibility:",_),!1}}function W(c){if(!c||!c.tagName)return!1;let _=new Set(["body","div","main","article","section","nav","header","footer"]),T=c.tagName.toLowerCase();return _.has(T)?!0:!new Set(["svg","script","style","link","meta","noscript","template"]).has(T)}function R(c){let _=g(c);return c.offsetWidth>0&&c.offsetHeight>0&&_?.visibility!=="hidden"&&_?.display!=="none"}function fe(c){if(!c||c.nodeType!==Node.ELEMENT_NODE||t.includes(c))return!1;if(n.includes(c))return!0;let _=c.tagName.toLowerCase(),T=g(c),x=new Set(["pointer","move","text","grab","grabbing","cell","copy","alias","all-scroll","col-resize","context-menu","crosshair","e-resize","ew-resize","help","n-resize","ne-resize","nesw-resize","ns-resize","nw-resize","nwse-resize","row-resize","s-resize","se-resize","sw-resize","vertical-text","w-resize","zoom-in","zoom-out"]),$=new Set(["not-allowed","no-drop","wait","progress","initial","inherit"]);function N(Q){return Q.tagName.toLowerCase()==="html"?!1:!!(T?.cursor&&x.has(T.cursor))}if(N(c))return!0;let y=new Set(["a","button","input","select","textarea","details","summary","label","option","optgroup","fieldset","legend"]),S=new Set(["disabled","readonly"]);if(y.has(_)){if(T?.cursor&&$.has(T.cursor))return!1;for(let Q of S)if(c.hasAttribute(Q)||c.getAttribute(Q)==="true"||c.getAttribute(Q)==="")return!1;return!(c.disabled||c.readOnly||c.inert)}let v=c.getAttribute("role"),I=c.getAttribute("aria-role");if(c.getAttribute("contenteditable")==="true"||c.isContentEditable||c.classList&&(c.classList.contains("button")||c.classList.contains("dropdown-toggle")||c.getAttribute("data-index")||c.getAttribute("data-toggle")==="dropdown"||c.getAttribute("aria-haspopup")==="true"))return!0;let ue=new Set(["button","menu","menubar","menuitem","menuitemradio","menuitemcheckbox","radio","checkbox","tab","switch","slider","spinbutton","combobox","searchbox","textbox","listbox","option","scrollbar"]);if(y.has(_)||v&&ue.has(v)||I&&ue.has(I))return!0;try{if(typeof getEventListeners=="function"){let _e=getEventListeners(c);for(let oe of["click","mousedown","mouseup","dblclick"])if(_e[oe]&&_e[oe].length>0)return!0}let Q=c?.ownerDocument?.defaultView?.getEventListenersForNode||window.getEventListenersForNode;if(typeof Q=="function"){let _e=Q(c);for(let oe of["click","mousedown","mouseup","keydown","keyup","submit","change","input","focus","blur"])for(let tt of _e)if(tt.type===oe)return!0}for(let _e of["onclick","onmousedown","onmouseup","ondblclick"])if(c.hasAttribute(_e)||typeof c[_e]=="function")return!0}catch{}return!!b(c)}function se(c){if(a===-1)return!0;let _=O(c);if(!_||_.length===0)return!1;let T=!1;for(let y of _)if(y.width>0&&y.height>0&&!(y.bottom<-a||y.top>window.innerHeight+a||y.right<-a||y.left>window.innerWidth+a)){T=!0;break}if(!T)return!1;if(c.ownerDocument!==window.document)return!0;let x=Array.from(_).find(y=>y.width>0&&y.height>0);if(!x)return!1;let $=c.getRootNode();if($ instanceof ShadowRoot){let y=x.left+x.width/2,S=x.top+x.height/2;try{let v=$.elementFromPoint(y,S);if(!v)return!1;let I=v;for(;I&&I!==$;){if(I===c)return!0;I=I.parentElement}return!1}catch{return!0}}let N=5;return[{x:x.left+x.width/2,y:x.top+x.height/2},{x:x.left+N,y:x.top+N},{x:x.right-N,y:x.bottom-N}].some(({x:y,y:S})=>{try{let v=document.elementFromPoint(y,S);if(!v)return!1;let I=v;for(;I&&I!==document.documentElement;){if(I===c)return!0;I=I.parentElement}return!1}catch{return!0}})}function te(c,_){if(_===-1)return!0;let T=c.getClientRects();if(!T||T.length===0){let x=f(c);return!x||x.width===0||x.height===0?!1:!(x.bottom<-_||x.top>window.innerHeight+_||x.right<-_||x.left>window.innerWidth+_)}for(let x of T)if(!(x.width===0||x.height===0)&&!(x.bottom<-_||x.top>window.innerHeight+_||x.right<-_||x.left>window.innerWidth+_))return!0;return!1}let Y=["aria-expanded","aria-checked","aria-selected","aria-pressed","aria-haspopup","aria-controls","aria-owns","aria-activedescendant","aria-valuenow","aria-valuetext","aria-valuemax","aria-valuemin","aria-autocomplete"];function ne(c){for(let _=0;_<Y.length;_++)if(c.hasAttribute(Y[_]))return!0;return!1}function Xe(c){if(!c||c.nodeType!==Node.ELEMENT_NODE)return!1;let _=c.tagName.toLowerCase();return new Set(["a","button","input","select","textarea","details","summary","label"]).has(_)?!0:c.hasAttribute("onclick")||c.hasAttribute("role")||c.hasAttribute("tabindex")||ne(c)||c.hasAttribute("data-action")||c.getAttribute("contenteditable")==="true"}let me=new Set(["a","button","input","select","textarea","summary","details","label","option","li"]),Ke=new Set(["button","link","menuitem","menuitemradio","menuitemcheckbox","radio","checkbox","tab","switch","slider","spinbutton","combobox","searchbox","textbox","listbox","listitem","treeitem","row","option","scrollbar"]);function Qe(c){if(!c||c.nodeType!==Node.ELEMENT_NODE||!R(c))return!1;let _=c.hasAttribute("role")||c.hasAttribute("tabindex")||c.hasAttribute("onclick")||typeof c.onclick=="function",T=/\b(btn|clickable|menu|item|entry|link)\b/i.test(c.className||""),x=!!c.closest('button,a,[role="button"],.menu,.dropdown,.list,.toolbar'),$=[...c.children].some(R),N=c.parentElement&&c.parentElement.isSameNode(document.body);return(fe(c)||_||T)&&$&&x&&!N}function et(c){if(!c||c.nodeType!==Node.ELEMENT_NODE)return!1;let _=c.tagName.toLowerCase(),T=c.getAttribute("role");if(_==="iframe"||me.has(_)||T&&Ke.has(T)||c.isContentEditable||c.getAttribute("contenteditable")==="true"||c.hasAttribute("data-testid")||c.hasAttribute("data-cy")||c.hasAttribute("data-test")||c.hasAttribute("onclick")||typeof c.onclick=="function"||ne(c))return!0;try{let x=c?.ownerDocument?.defaultView?.getEventListenersForNode||window.getEventListenersForNode;if(typeof x=="function"){let $=x(c);for(let N of["click","mousedown","mouseup","keydown","keyup","submit","change","input","focus","blur"])for(let y of $)if(y.type===N)return!0}if(["onmousedown","onmouseup","onkeydown","onkeyup","onsubmit","onchange","oninput","onfocus","onblur"].some($=>c.hasAttribute($)))return!0}catch{}return!!(Qe(c)||p.get(c)?.scrollable)}function ge(c,_,T,x){if(!c.isInteractive)return!1;let $=!1;return x?et(_)?$=!0:$=!1:$=!0,$&&(c.isInViewport=te(_,a),(c.isInViewport||a===-1)&&(c.highlightIndex=l++,i))?(s>=0?s===c.highlightIndex&&A(_,c.highlightIndex,T):A(_,c.highlightIndex,T),!0):!1}function re(c,_=null,T=!1){if(!c||c.id===P||c.nodeType!==Node.ELEMENT_NODE&&c.nodeType!==Node.TEXT_NODE||!c||c.id===P||c.dataset?.browserUseIgnore==="true"||c.dataset?.pageAgentIgnore==="true"||c.getAttribute&&c.getAttribute("aria-hidden")==="true")return null;if(c===document.body){let y={tagName:"body",attributes:{},xpath:"/body",children:[]};for(let v of c.childNodes){let I=re(v,_,!1);I&&y.children.push(I)}let S=`${C.current++}`;return w[S]=y,S}if(c.nodeType!==Node.ELEMENT_NODE&&c.nodeType!==Node.TEXT_NODE)return null;if(c.nodeType===Node.TEXT_NODE){let y=c.textContent?.trim();if(!y)return null;let S=c.parentElement;if(!S||S.tagName.toLowerCase()==="script")return null;let v=`${C.current++}`;return w[v]={type:"TEXT_NODE",text:y,isVisible:k(c)},v}if(c.nodeType===Node.ELEMENT_NODE&&!W(c))return null;if(a!==-1&&!c.shadowRoot){let y=f(c),S=g(c),v=S&&(S.position==="fixed"||S.position==="sticky"),I=c.offsetWidth>0||c.offsetHeight>0;if(!y||!v&&!I&&(y.bottom<-a||y.top>window.innerHeight+a||y.right<-a||y.left>window.innerWidth+a))return null}let x={tagName:c.tagName.toLowerCase(),attributes:{},children:[]};if(Xe(c)||c.tagName.toLowerCase()==="iframe"||c.tagName.toLowerCase()==="body"){let y=c.getAttributeNames?.()||[];for(let S of y){let v=c.getAttribute(S);x.attributes[S]=v}c.tagName.toLowerCase()==="input"&&(c.type==="checkbox"||c.type==="radio")&&(x.attributes.checked=c.checked?"true":"false")}let $=!1;if(c.nodeType===Node.ELEMENT_NODE&&(x.isVisible=R(c),x.isVisible)){x.isTopElement=se(c);let y=c.getAttribute("role"),S=y==="menu"||y==="menubar"||y==="listbox";if((x.isTopElement||S)&&(x.isInteractive=fe(c),$=ge(x,c,_,T),x.ref=c,x.isInteractive&&Object.keys(x.attributes).length===0)){let v=c.getAttributeNames?.()||[];for(let I of v){let ue=c.getAttribute(I);x.attributes[I]=ue}}}if(c.tagName){let y=c.tagName.toLowerCase();if(y==="iframe")try{let S=c.contentDocument||c.contentWindow?.document;if(S)for(let v of S.childNodes){let I=re(v,c,!1);I&&x.children.push(I)}}catch(S){console.warn("Unable to access iframe:",S)}else if(c.isContentEditable||c.getAttribute("contenteditable")==="true"||c.id==="tinymce"||c.classList.contains("mce-content-body")||y==="body"&&c.getAttribute("data-id")?.startsWith("mce_"))for(let S of c.childNodes){let v=re(S,_,$);v&&x.children.push(v)}else{if(c.shadowRoot){x.shadowRoot=!0;for(let S of c.shadowRoot.childNodes){let v=re(S,_,$);v&&x.children.push(v)}}for(let S of c.childNodes){let v=re(S,_,$||T);v&&x.children.push(v)}}}if(x.tagName==="a"&&x.children.length===0&&!x.attributes.href){let y=f(c);if(!(y&&y.width>0&&y.height>0||c.offsetWidth>0||c.offsetHeight>0))return null}x.extra=p.get(c)||null;let N=`${C.current++}`;return w[N]=x,N}let qt=re(document.body);return d.clearCache(),{rootId:qt,map:w}},mh=pl({cleanUpHighlights:()=>ze,flatTreeToString:()=>Ea,getAllTextTillNextClickableElement:()=>Sa,getElementTextMap:()=>Pa,getFlatTree:()=>za,getSelectorMap:()=>Ta,resolveViewportExpansion:()=>lr}),Tl=-1;function lr(e){return e??Tl}var Pl=new Set(["nav","menu","header","footer","aside","dialog"]),wa=new WeakMap;function za(e){let t=lr(e.viewportExpansion),n=[];for(let s of e.interactiveBlacklist||[])typeof s=="function"?n.push(s()):n.push(s);let r=[];for(let s of e.interactiveWhitelist||[])typeof s=="function"?r.push(s()):r.push(s);let o=Sl({doHighlightElements:!0,debugMode:!0,focusHighlightIndex:-1,viewportExpansion:t,interactiveBlacklist:n,interactiveWhitelist:r,highlightOpacity:e.highlightOpacity??0,highlightLabelOpacity:e.highlightLabelOpacity??.1}),i=window.location.href;for(let s in o.map){let a=o.map[s];if(a.isInteractive&&a.ref){let u=a.ref;wa.has(u)||(wa.set(u,i),a.isNew=!0)}}return o}var va=new Map;function Al(e){let t=va.get(e);if(!t){let n=e.replace(/[.+^${}()|[\]\\]/g,"\\$&");t=new RegExp(`^${n.replace(/\*/g,".*")}$`),va.set(e,t)}return t}function Il(e,t){let n={};for(let r of t)if(r.includes("*")){let o=Al(r);for(let i of Object.keys(e))o.test(i)&&e[i].trim()&&(n[i]=e[i].trim())}else{let o=e[r];o&&o.trim()&&(n[r]=o.trim())}return n}function Ea(e,t=[],n=!1){let r=["title","type","checked","name","role","value","placeholder","data-date-format","alt","aria-label","aria-expanded","data-state","aria-checked","id","for","target","aria-haspopup","aria-controls","aria-owns","contenteditable"],o=[...t,...r],i=(d,f)=>d.length>f?d.substring(0,f)+"...":d,s=d=>{let f=e.map[d];if(!f)return null;if(f.type==="TEXT_NODE"){let g=f;return{type:"text",text:g.text,isVisible:g.isVisible,parent:null,children:[]}}else{let g=f,O=[];if(g.children)for(let w of g.children){let C=s(w);C&&(C.parent=null,O.push(C))}return{type:"element",tagName:g.tagName,attributes:g.attributes??{},isVisible:g.isVisible??!1,isInteractive:g.isInteractive??!1,isTopElement:g.isTopElement??!1,isNew:g.isNew??!1,highlightIndex:g.highlightIndex,parent:null,children:O,extra:g.extra??{}}}},a=(d,f=null)=>{d.parent=f;for(let g of d.children)a(g,d)},u=s(e.rootId);if(!u)return"";a(u);let l=d=>{let f=d.parent;for(;f;){if(f.type==="element"&&f.highlightIndex!==void 0)return!0;f=f.parent}return!1},p=(d,f,g)=>{let O=f,w="	".repeat(f);if(d.type==="element"){let C=n&&d.tagName&&Pl.has(d.tagName);if(d.highlightIndex!==void 0){O+=1;let b=Sa(d),k="";if(o.length>0&&d.attributes){let R=Il(d.attributes,o),fe=Object.keys(R);if(fe.length>1){let se=new Set,te={};for(let Y of fe){let ne=R[Y];ne.length>5&&(ne in te?se.add(Y):te[ne]=Y)}for(let Y of se)delete R[Y]}R.role===d.tagName&&delete R.role;for(let se of["aria-label","placeholder","title"])R[se]&&R[se].toLowerCase().trim()===b.toLowerCase().trim()&&delete R[se];Object.keys(R).length>0&&(k=Object.entries(R).map(([se,te])=>`${se}=${i(te,20)}`).join(" "))}let W=`${w}${d.isNew?`*[${d.highlightIndex}]`:`[${d.highlightIndex}]`}<${d.tagName??""}`;if(k&&(W+=` ${k}`),d.extra&&d.extra.scrollable){let R="";d.extra.scrollData?.left&&(R+=`left=${d.extra.scrollData.left}, `),d.extra.scrollData?.top&&(R+=`top=${d.extra.scrollData.top}, `),d.extra.scrollData?.right&&(R+=`right=${d.extra.scrollData.right}, `),d.extra.scrollData?.bottom&&(R+=`bottom=${d.extra.scrollData.bottom}`),W+=` data-scrollable="${R}"`}if(b){let R=b.trim();k||(W+=" "),W+=`>${R}`}else k||(W+=" ");W+=" />",g.push(W)}let P=C&&d.highlightIndex===void 0,A=P?g.length:-1;P&&(g.push(`${w}<${d.tagName}>`),O+=1);for(let b of d.children)p(b,O,g);P&&(g.length===A+1?g.pop():g.push(`${w}</${d.tagName}>`))}else if(d.type==="text"){if(l(d))return;d.parent&&d.parent.type==="element"&&d.parent.isVisible&&d.parent.isTopElement&&g.push(`${w}${d.text??""}`)}},m=[];return p(u,0,m),m.join(`
`)}var Sa=(e,t=-1)=>{let n=[],r=(o,i)=>{if(!(t!==-1&&i>t)&&!(o.type==="element"&&o!==e&&o.highlightIndex!==void 0)){if(o.type==="text"&&o.text)n.push(o.text);else if(o.type==="element")for(let s of o.children)r(s,i+1)}};return r(e,0),n.join(`
`).trim()};function Ta(e){let t=new Map,n=Object.keys(e.map);for(let r of n){let o=e.map[r];o.isInteractive&&typeof o.highlightIndex=="number"&&t.set(o.highlightIndex,o)}return t}function Pa(e){let t=e.split(`
`).map(r=>r.trim()).filter(r=>r.length>0),n=new Map;for(let r of t){let o=/^\[(\d+)\]<[^>]+>([^<]*)/.exec(r);if(o){let i=parseInt(o[1],10);n.set(i,r)}}return n}function ze(){let e=window._highlightCleanupFunctions||[];for(let t of e)typeof t=="function"&&t();window._highlightCleanupFunctions=[]}window.addEventListener("popstate",()=>{ze()});window.addEventListener("hashchange",()=>{ze()});window.addEventListener("beforeunload",()=>{ze()});var ur=window.navigation;if(ur&&typeof ur.addEventListener=="function")ur.addEventListener("navigate",()=>{ze()});else{let e=window.location.href;setInterval(()=>{window.location.href!==e&&(e=window.location.href,ze())},500)}function Ol(){let e=window.innerWidth,t=window.innerHeight,n=Math.max(document.documentElement.scrollWidth,document.body.scrollWidth||0),r=Math.max(document.documentElement.scrollHeight,document.body.scrollHeight||0),o=window.scrollX||window.pageXOffset||document.documentElement.scrollLeft||0,i=window.scrollY||window.pageYOffset||document.documentElement.scrollTop||0,s=Math.max(0,r-(window.innerHeight+i)),a=Math.max(0,n-(window.innerWidth+o));return{viewport_width:e,viewport_height:t,page_width:n,page_height:r,scroll_x:o,scroll_y:i,pixels_above:i,pixels_below:s,pages_above:t>0?i/t:0,pages_below:t>0?s/t:0,total_pages:t>0?r/t:0,current_page_position:i/Math.max(1,r-t),pixels_left:o,pixels_right:a}}function Nl(e){let t=document.querySelectorAll('[data-reactroot], [data-reactid], [data-react-checksum], #root, #app, [id^="root-"], [id^="app-"], #adex-wrapper, #adex-root');for(let n of t)n.setAttribute("data-page-agent-not-interactive","true")}var Aa=class extends EventTarget{config;flatTree=null;selectorMap=new Map;elementTextMap=new Map;simplifiedHTML="<EMPTY>";lastTimeUpdate=0;isIndexed=!1;mask=null;maskReady=null;constructor(e={}){super(),this.config=e,Nl(this),e.enableMask&&this.initMask()}initMask(){this.maskReady===null&&(this.maskReady=(async()=>{let{SimulatorMask:e}=await Promise.resolve().then(()=>(ba(),_a));this.mask=new e})())}async getCurrentUrl(){return window.location.href}async getLastUpdateTime(){return this.lastTimeUpdate}async getBrowserState(){let e=window.location.href,t=document.title,n=Ol(),r=lr(this.config.viewportExpansion);await this.updateTree();let o=this.simplifiedHTML;return{url:e,title:t,header:`${`Current Page: [${t}](${e})`}
${`Page info: ${n.viewport_width}x${n.viewport_height}px viewport, ${n.page_width}x${n.page_height}px total page size, ${n.pages_above.toFixed(1)} pages above, ${n.pages_below.toFixed(1)} pages below, ${n.total_pages.toFixed(1)} total pages, at ${(n.current_page_position*100).toFixed(0)}% of page`}

${r===-1?"Interactive elements from top layer of the current page (full page):":"Interactive elements from top layer of the current page inside the viewport:"}

${n.pixels_above>4&&r!==-1?`... ${n.pixels_above} pixels above (${n.pages_above.toFixed(1)} pages) - scroll to see more ...`:"[Start of page]"}`,content:o,footer:n.pixels_below>4&&r!==-1?`... ${n.pixels_below} pixels below (${n.pages_below.toFixed(1)} pages) - scroll to see more ...`:"[End of page]"}}async updateTree(){this.dispatchEvent(new Event("beforeUpdate")),this.lastTimeUpdate=Date.now(),this.mask&&(this.mask.wrapper.style.pointerEvents="none"),ze();let e=[...this.config.interactiveBlacklist||[],...Array.from(document.querySelectorAll("[data-page-agent-not-interactive]"))];return this.flatTree=za({...this.config,interactiveBlacklist:e}),this.simplifiedHTML=Ea(this.flatTree,this.config.includeAttributes,this.config.keepSemanticTags),this.selectorMap.clear(),this.selectorMap=Ta(this.flatTree),this.elementTextMap.clear(),this.elementTextMap=Pa(this.simplifiedHTML),this.isIndexed=!0,this.mask&&(this.mask.wrapper.style.pointerEvents="auto"),this.dispatchEvent(new Event("afterUpdate")),this.simplifiedHTML}async cleanUpHighlights(){console.log("[PageController] cleanUpHighlights"),ze()}assertIndexed(){if(!this.isIndexed)throw new Error("DOM tree not indexed yet. Can not perform actions on elements.")}async clickElement(e){try{this.assertIndexed();let t=$t(this.selectorMap,e),n=this.elementTextMap.get(e);return await $a(t),gl(t)&&t.target==="_blank"?{success:!0,message:`\u2705 Clicked element (${n??e}). \u26A0\uFE0F Link opened in a new tab.`}:{success:!0,message:`\u2705 Clicked element (${n??e}).`}}catch(t){return{success:!1,message:`\u274C Failed to click element: ${t}`}}}async inputText(e,t){try{this.assertIndexed();let n=$t(this.selectorMap,e),r=this.elementTextMap.get(e);return await kl(n,t),{success:!0,message:`\u2705 Input text (${t}) into element (${r??e}).`}}catch(n){return{success:!1,message:`\u274C Failed to input text: ${n}`}}}async selectOption(e,t){try{this.assertIndexed();let n=$t(this.selectorMap,e),r=this.elementTextMap.get(e);return await $l(n,t),{success:!0,message:`\u2705 Selected option (${t}) in element (${r??e}).`}}catch(n){return{success:!1,message:`\u274C Failed to select option: ${n}`}}}async scroll(e){try{let{down:t,numPages:n,pixels:r,index:o}=e;return this.assertIndexed(),{success:!0,message:await zl((r??n*window.innerHeight)*(t?1:-1),o!==void 0?$t(this.selectorMap,o):null)}}catch(t){return{success:!1,message:`\u274C Failed to scroll: ${t}`}}}async scrollHorizontally(e){try{let{right:t,pixels:n,index:r}=e;return this.assertIndexed(),{success:!0,message:await El(n*(t?1:-1),r!==void 0?$t(this.selectorMap,r):null)}}catch(t){return{success:!1,message:`\u274C Failed to scroll horizontally: ${t}`}}}async executeJavascript(script,signal){try{let asyncFunction=eval(`(async (signal) => { ${script} })`),result=await asyncFunction(signal);return{success:!0,message:`\u2705 Executed JavaScript. Result: ${result}`}}catch(e){return{success:!1,message:`\u274C Error executing JavaScript: ${e}`}}}async showMask(){await this.maskReady,this.mask?.show()}async hideMask(){await this.maskReady,this.mask?.hide()}dispose(){ze(),this.flatTree=null,this.selectorMap.clear(),this.elementTextMap.clear(),this.simplifiedHTML="<EMPTY>",this.isIndexed=!1,this.mask?.dispose(),this.mask=null}};(function(){try{if(typeof document<"u"){var e=document.createElement("style");e.appendChild(document.createTextNode(`._wrapper_1tu05_1 {
	position: fixed;
	bottom: 100px;
	left: 50%;
	transform: translateX(-50%) translateY(20px);
	opacity: 0;
	z-index: 2147483642; /* \u6BD4 SimulatorMask \u9AD8\u4E00\u5C42 */
	box-sizing: border-box;

	overflow: visible;

	* {
		box-sizing: border-box;
	}

	--width: 360px;
	--height: 40px;
	--border-radius: 12px;

	--side-space: 12px; /* \u63A7\u5236\u680F\u4E24\u4FA7\u7684\u95F4\u8DDD */
	--history-width: calc(var(--width) - var(--side-space) * 2);

	--color-1: rgb(57, 182, 255);
	--color-2: rgb(189, 69, 251);
	--color-3: rgb(255, 87, 51);
	--color-4: rgb(255, 214, 0);

	width: var(--width);
	height: var(--height);

	transition: all 0.3s ease-in-out;

	/* \u54CD\u5E94\u5F0F\u8BBE\u8BA1 */
	@media (max-width: 480px) {
		width: calc(100vw - 40px);
		--width: calc(100vw - 40px);
	}

	._background_1tu05_39 {
		position: absolute;
		inset: -2px -8px;
		border-radius: calc(var(--border-radius) + 4px);
		filter: blur(16px);
		overflow: hidden;
		/* mix-blend-mode: lighten; */
		/* display: none; */

		&::before {
			content: '';
			z-index: -1;
			pointer-events: none;
			position: absolute;
			width: 100%;
			height: 100%;
			/* left: -100%; */
			left: 0;
			top: 0;

			background-image: linear-gradient(
				to bottom left,
				var(--color-1),
				var(--color-2),
				var(--color-1)
			);
			animation: _mask-running_1tu05_1 2s linear infinite;
		}
		&::after {
			content: '';
			z-index: -1;
			pointer-events: none;
			position: absolute;
			width: 100%;
			height: 100%;
			left: 0;
			top: 0;

			background-image: linear-gradient(
				to bottom left,
				var(--color-2),
				var(--color-1),
				var(--color-2)
			);
			animation: _mask-running_1tu05_1 2s linear infinite;
			animation-delay: 1s;
		}
	}
}

@keyframes _mask-running_1tu05_1 {
	from {
		transform: translateX(-100%);
	}
	to {
		transform: translateX(100%);
	}
}

/* \u63A7\u5236\u680F */
._header_1tu05_99 {
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 8px 12px;
	user-select: none;

	position: absolute;
	inset: 0;

	cursor: pointer;
	flex-shrink: 0; /* \u9632\u6B62 header \u88AB\u538B\u7F29 */

	background: rgba(0, 0, 0, 0.5);
	backdrop-filter: blur(10px);
	border-radius: var(--border-radius);
	background-clip: padding-box;

	box-shadow:
		0 0 0px 2px rgba(255, 255, 255, 0.4),
		0 0 5px 1px rgba(255, 255, 255, 0.3);

	._statusSection_1tu05_121 {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		min-height: 24px; /* \u786E\u4FDD\u5782\u76F4\u5C45\u4E2D */

		._indicator_1tu05_128 {
			width: 6px;
			height: 6px;
			border-radius: 50%;
			background: rgba(255, 255, 255, 0.5);
			flex-shrink: 0;
			animation: none; /* \u9ED8\u8BA4\u65E0\u52A8\u753B */

			/* \u8FD0\u884C\u72B6\u6001 - \u6709\u52A8\u753B */
			&._thinking_1tu05_137 {
				background: rgb(57, 182, 255);
				animation: _pulse_1tu05_1 0.8s ease-in-out infinite;
			}

			&._tool_executing_1tu05_142 {
				background: rgb(189, 69, 251);
				animation: _pulse_1tu05_1 0.6s ease-in-out infinite;
			}

			&._retry_1tu05_147 {
				background: rgb(255, 214, 0);
				animation: _retryPulse_1tu05_1 1s ease-in-out infinite;
			}

			/* \u9759\u6B62\u72B6\u6001 - \u65E0\u52A8\u753B */
			&._completed_1tu05_153,
			&._input_1tu05_154,
			&._output_1tu05_155 {
				background: rgb(34, 197, 94);
				animation: none;
			}

			&._error_1tu05_160 {
				background: rgb(239, 68, 68);
				animation: none;
			}
		}

		._statusText_1tu05_166 {
			color: white;
			font-size: 12px;
			line-height: 1;
			font-weight: 500;
			transition: all 0.3s ease-in-out;
			position: relative;
			overflow: hidden;
			display: flex;
			align-items: center;
			min-height: 24px; /* \u786E\u4FDD\u5782\u76F4\u5C45\u4E2D */

			&._fadeOut_1tu05_178 {
				animation: _statusTextFadeOut_1tu05_1 0.3s ease forwards;
			}

			&._fadeIn_1tu05_182 {
				animation: _statusTextFadeIn_1tu05_1 0.3s ease forwards;
			}
		}
	}

	._controls_1tu05_188 {
		display: flex;
		align-items: center;
		gap: 4px;

		._controlButton_1tu05_193 {
			width: 24px;
			height: 24px;
			border: none;
			border-radius: 4px;
			background: rgba(255, 255, 255, 0.1);
			color: white;
			cursor: pointer;
			display: flex;
			align-items: center;
			justify-content: center;
			font-size: 12px;
			line-height: 1;

			&:hover {
				background: rgba(255, 255, 255, 0.2);
			}
		}

		._stopButton_1tu05_212 {
			background: rgba(239, 68, 68, 0.2);
			color: rgb(255, 41, 41);
			font-weight: 600;

			&:hover {
				background: rgba(239, 68, 68, 0.3);
			}
		}
	}
}

@keyframes _statusTextFadeIn_1tu05_1 {
	0% {
		opacity: 0;
		transform: translateY(5px);
	}
	100% {
		opacity: 1;
		transform: translateY(0);
	}
}

@keyframes _statusTextFadeOut_1tu05_1 {
	0% {
		opacity: 1;
		transform: translateY(0);
	}
	100% {
		opacity: 0;
		transform: translateY(-5px);
	}
}

._historySectionWrapper_1tu05_246 {
	position: absolute;
	width: var(--history-width);
	bottom: var(--height);
	left: var(--side-space);
	z-index: -2;

	padding-top: 0px;
	visibility: collapse;
	overflow: hidden;

	transition: all 0.2s;

	background: rgba(2, 0, 20, 0.5);
	/* background: rgba(186, 186, 186, 0.2); */
	backdrop-filter: blur(10px);

	text-shadow: 0 0 1px rgba(0, 0, 0, 0.2);

	border-top-left-radius: calc(var(--border-radius) + 4px);
	border-top-right-radius: calc(var(--border-radius) + 4px);

	/* border: 2px solid rgba(255, 255, 255, 0.8); */
	border: 2px solid rgba(255, 255, 255, 0.4);
	box-shadow: 0 4px 16px rgba(0, 0, 0, 0.6);

	/* @media (prefers-color-scheme: dark) {
		box-shadow:
			0 8px 32px 0 rgba(0, 0, 0, 0.85),
			0 2px 12px 0 rgba(57, 182, 255, 0.1);
	} */

	._expanded_1tu05_278 & {
		padding-top: 8px;
		visibility: visible;
	}

	._historySection_1tu05_246 {
		position: relative;
		overflow-y: auto;
		overscroll-behavior: contain;
		scrollbar-width: none;
		max-height: 0;
		padding-inline: 8px;

		transition: max-height 0.2s;

		._expanded_1tu05_278 & {
			max-height: min(500px, calc(100vh - 200px - var(--height)));
		}

		._historyItem_1tu05_297 {
			/* backdrop-filter: blur(10px); */
			padding: 8px 10px;
			margin-bottom: 6px;
			background: linear-gradient(135deg, rgba(255, 255, 255, 0.08), rgba(255, 255, 255, 0.03));
			border-radius: 8px;
			border-left: 2px solid rgba(57, 182, 255, 0.5);
			font-size: 12px;
			color: white;
			/* color: black; */
			line-height: 1.3;
			position: relative;
			overflow: hidden;

			/* \u5FAE\u5999\u7684\u5185\u9634\u5F71 */
			box-shadow:
				inset 0 1px 0 rgba(255, 255, 255, 0.1),
				0 1px 3px rgba(0, 0, 0, 0.1);

			&::before {
				content: '';
				position: absolute;
				top: 0;
				left: 0;
				right: 0;
				height: 1px;
				background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.2), transparent);
			}

			&:hover {
				background: linear-gradient(135deg, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.06));
				/* transform: translateY(-1px); */
				box-shadow:
					inset 0 1px 0 rgba(255, 255, 255, 0.15),
					0 2px 4px rgba(0, 0, 0, 0.15);
			}

			&:last-child {
				margin-bottom: 10px;
			}

			&._completed_1tu05_153,
			&._input_1tu05_154,
			&._output_1tu05_155 {
				border-left-color: rgb(34, 197, 94);
				background: linear-gradient(135deg, rgba(34, 197, 94, 0.1), rgba(34, 197, 94, 0.05));
			}

			&._error_1tu05_160 {
				border-left-color: rgb(239, 68, 68);
				background: linear-gradient(135deg, rgba(239, 68, 68, 0.1), rgba(239, 68, 68, 0.05));
			}

			&._retry_1tu05_147 {
				border-left-color: rgb(255, 214, 0);
				background: linear-gradient(135deg, rgba(255, 214, 0, 0.1), rgba(255, 214, 0, 0.05));
			}

			&._observation_1tu05_355 {
				border-left-color: rgb(147, 51, 234);
				background: linear-gradient(135deg, rgba(147, 51, 234, 0.1), rgba(147, 51, 234, 0.05));
			}

			&._question_1tu05_360 {
				border-left-color: rgb(255, 159, 67);
				background: linear-gradient(135deg, rgba(255, 159, 67, 0.15), rgba(255, 159, 67, 0.08));
			}

			/* \u7A81\u51FA\u663E\u793A done \u6210\u529F\u7ED3\u679C */
			&._doneSuccess_1tu05_366 {
				background: linear-gradient(
					135deg,
					rgba(34, 197, 94, 0.25),
					rgba(34, 197, 94, 0.15),
					rgba(34, 197, 94, 0.08)
				);
				border: none;
				border-left: 4px solid rgb(34, 197, 94);
				box-shadow:
					0 4px 12px rgba(34, 197, 94, 0.3),
					inset 0 1px 0 rgba(255, 255, 255, 0.2),
					0 0 20px rgba(34, 197, 94, 0.1);
				font-weight: 600;
				color: rgb(220, 252, 231);
				padding: 10px 12px;
				margin-bottom: 8px;
				border-radius: 8px;
				position: relative;
				overflow: hidden;

				&::before {
					background: linear-gradient(90deg, transparent, rgba(34, 197, 94, 0.4), transparent);
				}

				&::after {
					content: '';
					position: absolute;
					top: 0;
					left: -100%;
					width: 100%;
					height: 100%;
					background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.1), transparent);
					animation: _shimmer_1tu05_1 2s ease-in-out infinite;
				}

				._historyContent_1tu05_402 {
					._statusIcon_1tu05_403 {
						font-size: 16px;
						animation: _celebrate_1tu05_1 0.8s ease-in-out;
						filter: drop-shadow(0 2px 4px rgba(34, 197, 94, 0.5));
					}
				}
			}

			/* \u7A81\u51FA\u663E\u793A done \u5931\u8D25\u7ED3\u679C */
			&._doneError_1tu05_412 {
				background: linear-gradient(
					135deg,
					rgba(239, 68, 68, 0.25),
					rgba(239, 68, 68, 0.15),
					rgba(239, 68, 68, 0.08)
				);
				border: none;
				border-left: 4px solid rgb(239, 68, 68);
				box-shadow:
					0 4px 12px rgba(239, 68, 68, 0.3),
					inset 0 1px 0 rgba(255, 255, 255, 0.2),
					0 0 20px rgba(239, 68, 68, 0.1);
				font-weight: 600;
				color: rgb(254, 226, 226);
				padding: 10px 12px;
				margin-bottom: 8px;
				border-radius: 8px;
				position: relative;
				overflow: hidden;

				&::before {
					background: linear-gradient(90deg, transparent, rgba(239, 68, 68, 0.4), transparent);
				}

				._historyContent_1tu05_402 {
					._statusIcon_1tu05_403 {
						font-size: 16px;
						filter: drop-shadow(0 2px 4px rgba(239, 68, 68, 0.5));
					}
				}
			}

			._historyContent_1tu05_402 {
				display: flex;
				align-items: flex-start;
				gap: 8px;

				word-break: break-all;
				white-space: pre-wrap;

				/* overflow-x: auto; */

				._statusIcon_1tu05_403 {
					font-size: 12px;
					flex-shrink: 0;
					line-height: 1;
					transition: all 0.3s ease;
				}

				._reflectionLines_1tu05_462 {
					display: flex;
					flex-direction: column;
					gap: 4px;
				}
			}

			._historyMeta_1tu05_469 {
				font-size: 10px;
				color: rgba(255, 255, 255, 0.6);
				/* color: rgb(61, 61, 61); */
				margin-top: 8px;
				line-height: 1;
			}
		}
	}
}

/* \u52A8\u753B\u5173\u952E\u5E27 - \u66F4\u5FEB\u7684\u95EA\u70C1 */
@keyframes _pulse_1tu05_1 {
	0%,
	100% {
		opacity: 1;
		transform: scale(1);
	}
	50% {
		opacity: 0.4;
		transform: scale(1.3);
	}
}

/* \u91CD\u8BD5\u52A8\u753B - \u65CB\u8F6C\u8109\u51B2 */
@keyframes _retryPulse_1tu05_1 {
	0%,
	100% {
		opacity: 1;
		transform: scale(1) rotate(0deg);
	}
	25% {
		opacity: 0.6;
		transform: scale(1.2) rotate(90deg);
	}
	50% {
		opacity: 0.8;
		transform: scale(1.1) rotate(180deg);
	}
	75% {
		opacity: 0.6;
		transform: scale(1.2) rotate(270deg);
	}
}

/* \u5E86\u795D\u52A8\u753B */
@keyframes _celebrate_1tu05_1 {
	0%,
	100% {
		transform: scale(1);
	}
	25% {
		transform: scale(1.2) rotate(-5deg);
	}
	75% {
		transform: scale(1.2) rotate(5deg);
	}
}

/* done \u5361\u7247\u7684\u5149\u6CFD\u6548\u679C */
@keyframes _shimmer_1tu05_1 {
	0% {
		left: -100%;
	}
	100% {
		left: 100%;
	}
}

/* \u8F93\u5165\u533A\u57DF\u6837\u5F0F */
._inputSectionWrapper_1tu05_539 {
	position: absolute;
	width: var(--history-width);
	top: var(--height);
	left: var(--side-space);
	z-index: -1;

	visibility: visible;
	overflow: hidden;

	height: 48px;

	transition: all 0.2s;

	background: rgba(186, 186, 186, 0.2);
	backdrop-filter: blur(10px);

	border-bottom-left-radius: calc(var(--border-radius) + 4px);
	border-bottom-right-radius: calc(var(--border-radius) + 4px);

	border: 2px solid rgba(255, 255, 255, 0.3);
	box-shadow: 0 1px 16px rgba(0, 0, 0, 0.4);

	&._hidden_1tu05_562 {
		visibility: collapse;
		height: 0;
	}

	._inputSection_1tu05_539 {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 8px 8px;

		._taskInput_1tu05_573 {
			flex: 1;
			background: rgba(255, 255, 255, 0.4);
			border: 1px solid rgba(255, 255, 255, 0.3);
			border-radius: 10px;
			padding-inline: 10px;
			color: rgb(20, 20, 20);
			font-size: 12px;
			height: 28px;
			line-height: 1;
			outline: none;
			transition: all 0.2s ease;

			/* text-shadow: 0 0 2px rgba(255, 255, 255, 0.8); */

			/* border-color: rgba(57, 182, 255, 0.3); */

			&::placeholder {
				color: rgb(53, 53, 53);
			}

			&:focus {
				background: rgba(255, 255, 255, 0.8);
				border-color: rgba(57, 182, 255, 0.6);
				box-shadow: 0 0 0 2px rgba(57, 182, 255, 0.2);
			}
		}
	}
}`)),document.head.appendChild(e)}}catch(t){console.error("vite-plugin-css-injected-by-js",t)}})();(function(){try{if(typeof document<"u"){var e=document.createElement("style");e.appendChild(document.createTextNode(`._wrapper_1tu05_1 {
	position: fixed;
	bottom: 100px;
	left: 50%;
	transform: translateX(-50%) translateY(20px);
	opacity: 0;
	z-index: 2147483642; /* \u6BD4 SimulatorMask \u9AD8\u4E00\u5C42 */
	box-sizing: border-box;

	overflow: visible;

	* {
		box-sizing: border-box;
	}

	--width: 360px;
	--height: 40px;
	--border-radius: 12px;

	--side-space: 12px; /* \u63A7\u5236\u680F\u4E24\u4FA7\u7684\u95F4\u8DDD */
	--history-width: calc(var(--width) - var(--side-space) * 2);

	--color-1: rgb(57, 182, 255);
	--color-2: rgb(189, 69, 251);
	--color-3: rgb(255, 87, 51);
	--color-4: rgb(255, 214, 0);

	width: var(--width);
	height: var(--height);

	transition: all 0.3s ease-in-out;

	/* \u54CD\u5E94\u5F0F\u8BBE\u8BA1 */
	@media (max-width: 480px) {
		width: calc(100vw - 40px);
		--width: calc(100vw - 40px);
	}

	._background_1tu05_39 {
		position: absolute;
		inset: -2px -8px;
		border-radius: calc(var(--border-radius) + 4px);
		filter: blur(16px);
		overflow: hidden;
		/* mix-blend-mode: lighten; */
		/* display: none; */

		&::before {
			content: '';
			z-index: -1;
			pointer-events: none;
			position: absolute;
			width: 100%;
			height: 100%;
			/* left: -100%; */
			left: 0;
			top: 0;

			background-image: linear-gradient(
				to bottom left,
				var(--color-1),
				var(--color-2),
				var(--color-1)
			);
			animation: _mask-running_1tu05_1 2s linear infinite;
		}
		&::after {
			content: '';
			z-index: -1;
			pointer-events: none;
			position: absolute;
			width: 100%;
			height: 100%;
			left: 0;
			top: 0;

			background-image: linear-gradient(
				to bottom left,
				var(--color-2),
				var(--color-1),
				var(--color-2)
			);
			animation: _mask-running_1tu05_1 2s linear infinite;
			animation-delay: 1s;
		}
	}
}

@keyframes _mask-running_1tu05_1 {
	from {
		transform: translateX(-100%);
	}
	to {
		transform: translateX(100%);
	}
}

/* \u63A7\u5236\u680F */
._header_1tu05_99 {
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 8px 12px;
	user-select: none;

	position: absolute;
	inset: 0;

	cursor: pointer;
	flex-shrink: 0; /* \u9632\u6B62 header \u88AB\u538B\u7F29 */

	background: rgba(0, 0, 0, 0.5);
	backdrop-filter: blur(10px);
	border-radius: var(--border-radius);
	background-clip: padding-box;

	box-shadow:
		0 0 0px 2px rgba(255, 255, 255, 0.4),
		0 0 5px 1px rgba(255, 255, 255, 0.3);

	._statusSection_1tu05_121 {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		min-height: 24px; /* \u786E\u4FDD\u5782\u76F4\u5C45\u4E2D */

		._indicator_1tu05_128 {
			width: 6px;
			height: 6px;
			border-radius: 50%;
			background: rgba(255, 255, 255, 0.5);
			flex-shrink: 0;
			animation: none; /* \u9ED8\u8BA4\u65E0\u52A8\u753B */

			/* \u8FD0\u884C\u72B6\u6001 - \u6709\u52A8\u753B */
			&._thinking_1tu05_137 {
				background: rgb(57, 182, 255);
				animation: _pulse_1tu05_1 0.8s ease-in-out infinite;
			}

			&._tool_executing_1tu05_142 {
				background: rgb(189, 69, 251);
				animation: _pulse_1tu05_1 0.6s ease-in-out infinite;
			}

			&._retry_1tu05_147 {
				background: rgb(255, 214, 0);
				animation: _retryPulse_1tu05_1 1s ease-in-out infinite;
			}

			/* \u9759\u6B62\u72B6\u6001 - \u65E0\u52A8\u753B */
			&._completed_1tu05_153,
			&._input_1tu05_154,
			&._output_1tu05_155 {
				background: rgb(34, 197, 94);
				animation: none;
			}

			&._error_1tu05_160 {
				background: rgb(239, 68, 68);
				animation: none;
			}
		}

		._statusText_1tu05_166 {
			color: white;
			font-size: 12px;
			line-height: 1;
			font-weight: 500;
			transition: all 0.3s ease-in-out;
			position: relative;
			overflow: hidden;
			display: flex;
			align-items: center;
			min-height: 24px; /* \u786E\u4FDD\u5782\u76F4\u5C45\u4E2D */

			&._fadeOut_1tu05_178 {
				animation: _statusTextFadeOut_1tu05_1 0.3s ease forwards;
			}

			&._fadeIn_1tu05_182 {
				animation: _statusTextFadeIn_1tu05_1 0.3s ease forwards;
			}
		}
	}

	._controls_1tu05_188 {
		display: flex;
		align-items: center;
		gap: 4px;

		._controlButton_1tu05_193 {
			width: 24px;
			height: 24px;
			border: none;
			border-radius: 4px;
			background: rgba(255, 255, 255, 0.1);
			color: white;
			cursor: pointer;
			display: flex;
			align-items: center;
			justify-content: center;
			font-size: 12px;
			line-height: 1;

			&:hover {
				background: rgba(255, 255, 255, 0.2);
			}
		}

		._stopButton_1tu05_212 {
			background: rgba(239, 68, 68, 0.2);
			color: rgb(255, 41, 41);
			font-weight: 600;

			&:hover {
				background: rgba(239, 68, 68, 0.3);
			}
		}
	}
}

@keyframes _statusTextFadeIn_1tu05_1 {
	0% {
		opacity: 0;
		transform: translateY(5px);
	}
	100% {
		opacity: 1;
		transform: translateY(0);
	}
}

@keyframes _statusTextFadeOut_1tu05_1 {
	0% {
		opacity: 1;
		transform: translateY(0);
	}
	100% {
		opacity: 0;
		transform: translateY(-5px);
	}
}

._historySectionWrapper_1tu05_246 {
	position: absolute;
	width: var(--history-width);
	bottom: var(--height);
	left: var(--side-space);
	z-index: -2;

	padding-top: 0px;
	visibility: collapse;
	overflow: hidden;

	transition: all 0.2s;

	background: rgba(2, 0, 20, 0.5);
	/* background: rgba(186, 186, 186, 0.2); */
	backdrop-filter: blur(10px);

	text-shadow: 0 0 1px rgba(0, 0, 0, 0.2);

	border-top-left-radius: calc(var(--border-radius) + 4px);
	border-top-right-radius: calc(var(--border-radius) + 4px);

	/* border: 2px solid rgba(255, 255, 255, 0.8); */
	border: 2px solid rgba(255, 255, 255, 0.4);
	box-shadow: 0 4px 16px rgba(0, 0, 0, 0.6);

	/* @media (prefers-color-scheme: dark) {
		box-shadow:
			0 8px 32px 0 rgba(0, 0, 0, 0.85),
			0 2px 12px 0 rgba(57, 182, 255, 0.1);
	} */

	._expanded_1tu05_278 & {
		padding-top: 8px;
		visibility: visible;
	}

	._historySection_1tu05_246 {
		position: relative;
		overflow-y: auto;
		overscroll-behavior: contain;
		scrollbar-width: none;
		max-height: 0;
		padding-inline: 8px;

		transition: max-height 0.2s;

		._expanded_1tu05_278 & {
			max-height: min(500px, calc(100vh - 200px - var(--height)));
		}

		._historyItem_1tu05_297 {
			/* backdrop-filter: blur(10px); */
			padding: 8px 10px;
			margin-bottom: 6px;
			background: linear-gradient(135deg, rgba(255, 255, 255, 0.08), rgba(255, 255, 255, 0.03));
			border-radius: 8px;
			border-left: 2px solid rgba(57, 182, 255, 0.5);
			font-size: 12px;
			color: white;
			/* color: black; */
			line-height: 1.3;
			position: relative;
			overflow: hidden;

			/* \u5FAE\u5999\u7684\u5185\u9634\u5F71 */
			box-shadow:
				inset 0 1px 0 rgba(255, 255, 255, 0.1),
				0 1px 3px rgba(0, 0, 0, 0.1);

			&::before {
				content: '';
				position: absolute;
				top: 0;
				left: 0;
				right: 0;
				height: 1px;
				background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.2), transparent);
			}

			&:hover {
				background: linear-gradient(135deg, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.06));
				/* transform: translateY(-1px); */
				box-shadow:
					inset 0 1px 0 rgba(255, 255, 255, 0.15),
					0 2px 4px rgba(0, 0, 0, 0.15);
			}

			&:last-child {
				margin-bottom: 10px;
			}

			&._completed_1tu05_153,
			&._input_1tu05_154,
			&._output_1tu05_155 {
				border-left-color: rgb(34, 197, 94);
				background: linear-gradient(135deg, rgba(34, 197, 94, 0.1), rgba(34, 197, 94, 0.05));
			}

			&._error_1tu05_160 {
				border-left-color: rgb(239, 68, 68);
				background: linear-gradient(135deg, rgba(239, 68, 68, 0.1), rgba(239, 68, 68, 0.05));
			}

			&._retry_1tu05_147 {
				border-left-color: rgb(255, 214, 0);
				background: linear-gradient(135deg, rgba(255, 214, 0, 0.1), rgba(255, 214, 0, 0.05));
			}

			&._observation_1tu05_355 {
				border-left-color: rgb(147, 51, 234);
				background: linear-gradient(135deg, rgba(147, 51, 234, 0.1), rgba(147, 51, 234, 0.05));
			}

			&._question_1tu05_360 {
				border-left-color: rgb(255, 159, 67);
				background: linear-gradient(135deg, rgba(255, 159, 67, 0.15), rgba(255, 159, 67, 0.08));
			}

			/* \u7A81\u51FA\u663E\u793A done \u6210\u529F\u7ED3\u679C */
			&._doneSuccess_1tu05_366 {
				background: linear-gradient(
					135deg,
					rgba(34, 197, 94, 0.25),
					rgba(34, 197, 94, 0.15),
					rgba(34, 197, 94, 0.08)
				);
				border: none;
				border-left: 4px solid rgb(34, 197, 94);
				box-shadow:
					0 4px 12px rgba(34, 197, 94, 0.3),
					inset 0 1px 0 rgba(255, 255, 255, 0.2),
					0 0 20px rgba(34, 197, 94, 0.1);
				font-weight: 600;
				color: rgb(220, 252, 231);
				padding: 10px 12px;
				margin-bottom: 8px;
				border-radius: 8px;
				position: relative;
				overflow: hidden;

				&::before {
					background: linear-gradient(90deg, transparent, rgba(34, 197, 94, 0.4), transparent);
				}

				&::after {
					content: '';
					position: absolute;
					top: 0;
					left: -100%;
					width: 100%;
					height: 100%;
					background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.1), transparent);
					animation: _shimmer_1tu05_1 2s ease-in-out infinite;
				}

				._historyContent_1tu05_402 {
					._statusIcon_1tu05_403 {
						font-size: 16px;
						animation: _celebrate_1tu05_1 0.8s ease-in-out;
						filter: drop-shadow(0 2px 4px rgba(34, 197, 94, 0.5));
					}
				}
			}

			/* \u7A81\u51FA\u663E\u793A done \u5931\u8D25\u7ED3\u679C */
			&._doneError_1tu05_412 {
				background: linear-gradient(
					135deg,
					rgba(239, 68, 68, 0.25),
					rgba(239, 68, 68, 0.15),
					rgba(239, 68, 68, 0.08)
				);
				border: none;
				border-left: 4px solid rgb(239, 68, 68);
				box-shadow:
					0 4px 12px rgba(239, 68, 68, 0.3),
					inset 0 1px 0 rgba(255, 255, 255, 0.2),
					0 0 20px rgba(239, 68, 68, 0.1);
				font-weight: 600;
				color: rgb(254, 226, 226);
				padding: 10px 12px;
				margin-bottom: 8px;
				border-radius: 8px;
				position: relative;
				overflow: hidden;

				&::before {
					background: linear-gradient(90deg, transparent, rgba(239, 68, 68, 0.4), transparent);
				}

				._historyContent_1tu05_402 {
					._statusIcon_1tu05_403 {
						font-size: 16px;
						filter: drop-shadow(0 2px 4px rgba(239, 68, 68, 0.5));
					}
				}
			}

			._historyContent_1tu05_402 {
				display: flex;
				align-items: flex-start;
				gap: 8px;

				word-break: break-all;
				white-space: pre-wrap;

				/* overflow-x: auto; */

				._statusIcon_1tu05_403 {
					font-size: 12px;
					flex-shrink: 0;
					line-height: 1;
					transition: all 0.3s ease;
				}

				._reflectionLines_1tu05_462 {
					display: flex;
					flex-direction: column;
					gap: 4px;
				}
			}

			._historyMeta_1tu05_469 {
				font-size: 10px;
				color: rgba(255, 255, 255, 0.6);
				/* color: rgb(61, 61, 61); */
				margin-top: 8px;
				line-height: 1;
			}
		}
	}
}

/* \u52A8\u753B\u5173\u952E\u5E27 - \u66F4\u5FEB\u7684\u95EA\u70C1 */
@keyframes _pulse_1tu05_1 {
	0%,
	100% {
		opacity: 1;
		transform: scale(1);
	}
	50% {
		opacity: 0.4;
		transform: scale(1.3);
	}
}

/* \u91CD\u8BD5\u52A8\u753B - \u65CB\u8F6C\u8109\u51B2 */
@keyframes _retryPulse_1tu05_1 {
	0%,
	100% {
		opacity: 1;
		transform: scale(1) rotate(0deg);
	}
	25% {
		opacity: 0.6;
		transform: scale(1.2) rotate(90deg);
	}
	50% {
		opacity: 0.8;
		transform: scale(1.1) rotate(180deg);
	}
	75% {
		opacity: 0.6;
		transform: scale(1.2) rotate(270deg);
	}
}

/* \u5E86\u795D\u52A8\u753B */
@keyframes _celebrate_1tu05_1 {
	0%,
	100% {
		transform: scale(1);
	}
	25% {
		transform: scale(1.2) rotate(-5deg);
	}
	75% {
		transform: scale(1.2) rotate(5deg);
	}
}

/* done \u5361\u7247\u7684\u5149\u6CFD\u6548\u679C */
@keyframes _shimmer_1tu05_1 {
	0% {
		left: -100%;
	}
	100% {
		left: 100%;
	}
}

/* \u8F93\u5165\u533A\u57DF\u6837\u5F0F */
._inputSectionWrapper_1tu05_539 {
	position: absolute;
	width: var(--history-width);
	top: var(--height);
	left: var(--side-space);
	z-index: -1;

	visibility: visible;
	overflow: hidden;

	height: 48px;

	transition: all 0.2s;

	background: rgba(186, 186, 186, 0.2);
	backdrop-filter: blur(10px);

	border-bottom-left-radius: calc(var(--border-radius) + 4px);
	border-bottom-right-radius: calc(var(--border-radius) + 4px);

	border: 2px solid rgba(255, 255, 255, 0.3);
	box-shadow: 0 1px 16px rgba(0, 0, 0, 0.4);

	&._hidden_1tu05_562 {
		visibility: collapse;
		height: 0;
	}

	._inputSection_1tu05_539 {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 8px 8px;

		._taskInput_1tu05_573 {
			flex: 1;
			background: rgba(255, 255, 255, 0.4);
			border: 1px solid rgba(255, 255, 255, 0.3);
			border-radius: 10px;
			padding-inline: 10px;
			color: rgb(20, 20, 20);
			font-size: 12px;
			height: 28px;
			line-height: 1;
			outline: none;
			transition: all 0.2s ease;

			/* text-shadow: 0 0 2px rgba(255, 255, 255, 0.8); */

			/* border-color: rgba(57, 182, 255, 0.3); */

			&::placeholder {
				color: rgb(53, 53, 53);
			}

			&:focus {
				background: rgba(255, 255, 255, 0.8);
				border-color: rgba(57, 182, 255, 0.6);
				box-shadow: 0 0 0 2px rgba(57, 182, 255, 0.2);
			}
		}
	}
}`)),document.head.appendChild(e)}}catch(t){console.error("vite-plugin-css-injected-by-js",t)}})();var Ia={"en-US":{ui:{panel:{ready:"Ready",thinking:"Thinking...",taskInput:"Enter new task, describe steps in detail, press Enter to submit",userAnswerPrompt:"Please answer the question above, press Enter to submit",taskTerminated:"Task terminated",taskCompleted:"Task completed",userAnswer:"User answer: {{input}}",question:"Question: {{question}}",waitingPlaceholder:"Waiting for task to start...",stop:"Stop",close:"Close",expand:"Expand history",collapse:"Collapse history",step:"Step {{number}}"},tools:{clicking:"Clicking element [{{index}}]...",inputting:"Inputting text to element [{{index}}]...",selecting:'Selecting option "{{text}}"...',scrolling:"Scrolling page...",waiting:"Waiting {{seconds}} seconds...",askingUser:"Asking user...",done:"Task done",clicked:"\u{1F5B1}\uFE0F Clicked element [{{index}}]",inputted:'\u2328\uFE0F Inputted text "{{text}}"',selected:'\u2611\uFE0F Selected option "{{text}}"',scrolled:"\u{1F6DE} Page scrolled",waited:"\u231B\uFE0F Wait completed",executing:"Executing {{toolName}}...",resultSuccess:"success",resultFailure:"failed",resultError:"error"},errors:{elementNotFound:"No interactive element found at index {{index}}",taskRequired:"Task description is required",executionFailed:"Task execution failed",notInputElement:"Element is not an input or textarea",notSelectElement:"Element is not a select element",optionNotFound:'Option "{{text}}" not found'}}},"zh-CN":{ui:{panel:{ready:"\u51C6\u5907\u5C31\u7EEA",thinking:"\u6B63\u5728\u601D\u8003...",taskInput:"\u8F93\u5165\u65B0\u4EFB\u52A1\uFF0C\u8BE6\u7EC6\u63CF\u8FF0\u6B65\u9AA4\uFF0C\u56DE\u8F66\u63D0\u4EA4",userAnswerPrompt:"\u8BF7\u56DE\u7B54\u4E0A\u9762\u95EE\u9898\uFF0C\u56DE\u8F66\u63D0\u4EA4",taskTerminated:"\u4EFB\u52A1\u5DF2\u7EC8\u6B62",taskCompleted:"\u4EFB\u52A1\u7ED3\u675F",userAnswer:"\u7528\u6237\u56DE\u7B54: {{input}}",question:"\u8BE2\u95EE: {{question}}",waitingPlaceholder:"\u7B49\u5F85\u4EFB\u52A1\u5F00\u59CB...",stop:"\u7EC8\u6B62",close:"\u5173\u95ED",expand:"\u5C55\u5F00\u5386\u53F2",collapse:"\u6536\u8D77\u5386\u53F2",step:"\u6B65\u9AA4 {{number}}"},tools:{clicking:"\u6B63\u5728\u70B9\u51FB\u5143\u7D20 [{{index}}]...",inputting:"\u6B63\u5728\u8F93\u5165\u6587\u672C\u5230\u5143\u7D20 [{{index}}]...",selecting:'\u6B63\u5728\u9009\u62E9\u9009\u9879 "{{text}}"...',scrolling:"\u6B63\u5728\u6EDA\u52A8\u9875\u9762...",waiting:"\u7B49\u5F85 {{seconds}} \u79D2...",askingUser:"\u6B63\u5728\u8BE2\u95EE\u7528\u6237...",done:"\u7ED3\u675F\u4EFB\u52A1",clicked:"\u{1F5B1}\uFE0F \u5DF2\u70B9\u51FB\u5143\u7D20 [{{index}}]",inputted:'\u2328\uFE0F \u5DF2\u8F93\u5165\u6587\u672C "{{text}}"',selected:'\u2611\uFE0F \u5DF2\u9009\u62E9\u9009\u9879 "{{text}}"',scrolled:"\u{1F6DE} \u9875\u9762\u6EDA\u52A8\u5B8C\u6210",waited:"\u231B\uFE0F \u7B49\u5F85\u5B8C\u6210",executing:"\u6B63\u5728\u6267\u884C {{toolName}}...",resultSuccess:"\u6210\u529F",resultFailure:"\u5931\u8D25",resultError:"\u9519\u8BEF"},errors:{elementNotFound:"\u672A\u627E\u5230\u7D22\u5F15\u4E3A {{index}} \u7684\u4EA4\u4E92\u5143\u7D20",taskRequired:"\u4EFB\u52A1\u63CF\u8FF0\u4E0D\u80FD\u4E3A\u7A7A",executionFailed:"\u4EFB\u52A1\u6267\u884C\u5931\u8D25",notInputElement:"\u5143\u7D20\u4E0D\u662F\u8F93\u5165\u6846\u6216\u6587\u672C\u57DF",notSelectElement:"\u5143\u7D20\u4E0D\u662F\u9009\u62E9\u6846",optionNotFound:'\u672A\u627E\u5230\u9009\u9879 "{{text}}"'}}}},Cl=class{language;translations;constructor(e="en-US"){this.language=e in Ia?e:"en-US",this.translations=Ia[this.language]}t(e,t){let n=this.getNestedValue(this.translations,e);return n?t?this.interpolate(n,t):n:(console.warn(`Translation key "${e}" not found for language "${this.language}"`),e)}getNestedValue(e,t){return t.split(".").reduce((n,r)=>n?.[r],e)}interpolate(e,t){return e.replace(/\{\{(\w+)\}\}/g,(n,r)=>t[r]!=null?t[r].toString():n)}getLanguage(){return this.language}};function Oa(e,t){return e.length>t?e.substring(0,t)+"...":e}function Na(e){return e.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;").replace(/"/g,"&quot;").replace(/'/g,"&#039;")}var E={wrapper:"_wrapper_1tu05_1","mask-running":"_mask-running_1tu05_1",background:"_background_1tu05_39",header:"_header_1tu05_99",pulse:"_pulse_1tu05_1",retryPulse:"_retryPulse_1tu05_1",statusTextFadeOut:"_statusTextFadeOut_1tu05_1",statusTextFadeIn:"_statusTextFadeIn_1tu05_1",statusSection:"_statusSection_1tu05_121",indicator:"_indicator_1tu05_128",thinking:"_thinking_1tu05_137",tool_executing:"_tool_executing_1tu05_142",retry:"_retry_1tu05_147",completed:"_completed_1tu05_153",input:"_input_1tu05_154",output:"_output_1tu05_155",error:"_error_1tu05_160",statusText:"_statusText_1tu05_166",fadeOut:"_fadeOut_1tu05_178",fadeIn:"_fadeIn_1tu05_182",controls:"_controls_1tu05_188",controlButton:"_controlButton_1tu05_193",stopButton:"_stopButton_1tu05_212",historySectionWrapper:"_historySectionWrapper_1tu05_246",shimmer:"_shimmer_1tu05_1",celebrate:"_celebrate_1tu05_1",expanded:"_expanded_1tu05_278",historySection:"_historySection_1tu05_246",historyItem:"_historyItem_1tu05_297",observation:"_observation_1tu05_355",question:"_question_1tu05_360",doneSuccess:"_doneSuccess_1tu05_366",historyContent:"_historyContent_1tu05_402",statusIcon:"_statusIcon_1tu05_403",doneError:"_doneError_1tu05_412",reflectionLines:"_reflectionLines_1tu05_462",historyMeta:"_historyMeta_1tu05_469",inputSectionWrapper:"_inputSectionWrapper_1tu05_539",hidden:"_hidden_1tu05_562",inputSection:"_inputSection_1tu05_539",taskInput:"_taskInput_1tu05_573"};function he({icon:e,content:t,meta:n,type:r}){let o=r?E[r]:"",i=Array.isArray(t)?`<div class="${E.reflectionLines}">${t.map(s=>`<span>${Na(s)}</span>`).join("")}</div>`:`<span>${Na(t)}</span>`;return`
		<div class="${E.historyItem} ${o}">
			<div class="${E.historyContent}">
				<span class="${E.statusIcon}">${e}</span>
				${i}
			</div>
			${n?`<div class="${E.historyMeta}">${n}</div>`:""}
		</div>
	`}function Zl(e){let t=[];return e.evaluation_previous_goal&&t.push(`\u{1F50D} ${e.evaluation_previous_goal}`),e.memory&&t.push(`\u{1F4BE} ${e.memory}`),e.next_goal&&t.push(`\u{1F3AF} ${e.next_goal}`),t}var Ca=class{#n;#o;#t;#i;#c;#a;#s;#l;#e;#p;#h=!1;#r;#f=null;#d=!1;#m=null;#u=null;#x=!1;#w=()=>this.#I();#v=()=>this.#O();#k=e=>this.#N(e.detail);#$=()=>this.dispose();get wrapper(){return this.#n}constructor(e,t={}){this.#e=e,this.#p=t,this.#r=new Cl(t.language??"en-US"),this.#e.onAskUser=(n,r)=>this.#C(n,r?.signal),this.#n=this.#F(),this.#o=this.#n.querySelector(`.${E.indicator}`),this.#t=this.#n.querySelector(`.${E.statusText}`),this.#i=this.#n.querySelector(`.${E.historySection}`),this.#c=this.#n.querySelector(`.${E.expandButton}`),this.#a=this.#n.querySelector(`.${E.stopButton}`),this.#s=this.#n.querySelector(`.${E.inputSectionWrapper}`),this.#l=this.#n.querySelector(`.${E.taskInput}`),this.#e.addEventListener("statuschange",this.#w),this.#e.addEventListener("historychange",this.#v),this.#e.addEventListener("activity",this.#k),this.#e.addEventListener("dispose",this.#$),this.#D(),this.#j(),this.#_(),this.hide()}#I(){let e=this.#e.status,t=e==="completed"&&this.#e.lastResult?.success===!1;this.#g(t?"error":e),e==="running"?(this.#a.textContent="\u25A0",this.#a.title=this.#r.t("ui.panel.stop")):(this.#a.textContent="X",this.#a.title=this.#r.t("ui.panel.close")),e==="running"&&(this.show(),this.#S()),(e==="completed"||e==="error"||e==="stopped")&&(this.#h||this.#b(),this.#M()&&this.#_())}#O(){this.#A()}#N(e){switch(e.type){case"thinking":this.#u=this.#r.t("ui.panel.thinking"),this.#g("thinking");break;case"executing":this.#u=this.#E(e.tool,e.input),this.#g("executing");break;case"executed":this.#u=Oa(e.output,50);break;case"retrying":this.#u=`Retrying (${e.attempt}/${e.maxAttempts})`,this.#g("retrying");break;case"error":this.#u=Oa(e.message,50),this.#g("error");break}}#C(e,t){return new Promise((n,r)=>{this.#d=!0,this.#f=n,this.#h||this.#b();let o=document.createElement("div");o.innerHTML=he({icon:"\u2753",content:`Question: ${e}`,type:"question"});let i=o.firstElementChild;i.setAttribute("data-temp-card","true"),this.#i.appendChild(i),this.#P(),this.#_(this.#r.t("ui.panel.userAnswerPrompt")),t?.addEventListener("abort",()=>{this.#z(),this.#d=!1,this.#f=null,r(t.reason)},{once:!0})})}#z(){Array.from(this.#i.children).forEach(e=>{e.getAttribute("data-temp-card")==="true"&&e.remove()})}show(){this.wrapper.style.display="block",this.wrapper.offsetHeight,this.wrapper.style.opacity="1",this.wrapper.style.transform="translateX(-50%) translateY(0)"}hide(){this.wrapper.style.opacity="0",this.wrapper.style.transform="translateX(-50%) translateY(20px)",this.wrapper.style.display="none"}reset(){this.#t.textContent=this.#r.t("ui.panel.ready"),this.#g("thinking"),this.#A(),this.#y(),this.#d=!1,this.#f=null,this.#_()}expand(){this.#b()}collapse(){this.#y()}dispose(){this.#e.removeEventListener("statuschange",this.#w),this.#e.removeEventListener("historychange",this.#v),this.#e.removeEventListener("activity",this.#k),this.#e.removeEventListener("dispose",this.#$),this.#d=!1,this.#U(),this.wrapper.remove()}#E(e,t){let n=t;switch(e){case"click_element_by_index":return this.#r.t("ui.tools.clicking",{index:n.index});case"input_text":return this.#r.t("ui.tools.inputting",{index:n.index});case"select_dropdown_option":return this.#r.t("ui.tools.selecting",{text:n.text});case"scroll":return this.#r.t("ui.tools.scrolling");case"wait":return this.#r.t("ui.tools.waiting",{seconds:n.seconds});case"ask_user":return this.#r.t("ui.tools.askingUser");case"done":return this.#r.t("ui.tools.done");default:return this.#r.t("ui.tools.executing",{toolName:e})}}#Z(){this.#e.status==="running"?this.#e.stop():this.#e.dispose()}#R(){let e=this.#l.value.trim();e&&(this.#S(),this.#d?this.#L(e):this.#e.execute(e))}#L(e){this.#z(),this.#d=!1,this.#f&&(this.#f(e),this.#f=null)}#_(e){this.#l.value="",this.#l.placeholder=e||this.#r.t("ui.panel.taskInput"),this.#s.classList.remove(E.hidden),setTimeout(()=>{this.#l.focus()},100)}#S(){this.#s.classList.add(E.hidden)}#M(){if(this.#d||this.#e.history.length===0)return!0;let e=this.#e.status;return e==="completed"||e==="error"||e==="stopped"?this.#p.promptForNextTask??!0:!1}#F(){let t=document.createElement("div");return t.id="page-agent-runtime_agent-panel",t.className=E.wrapper,t.setAttribute("data-browser-use-ignore","true"),t.setAttribute("data-page-agent-ignore","true"),t.innerHTML=`
			<div class="${E.background}"></div>
			<div class="${E.historySectionWrapper}">
				<div class="${E.historySection}">
					<div class="${E.historyItem}">
						<div class="${E.historyContent}">
							<span class="${E.statusIcon}">\u{1F9E0}</span>
							<span>${this.#r.t("ui.panel.waitingPlaceholder")}</span>
						</div>
					</div>
				</div>
			</div>
			<div class="${E.header}">
				<div class="${E.statusSection}">
					<div class="${E.indicator} ${E.thinking}"></div>
					<div class="${E.statusText}">${this.#r.t("ui.panel.ready")}</div>
				</div>
				<div class="${E.controls}">
					<button class="${E.controlButton} ${E.expandButton}" title="${this.#r.t("ui.panel.expand")}">
						\u25BC
					</button>
					<button class="${E.controlButton} ${E.stopButton}" title="${this.#r.t("ui.panel.close")}">
						X
					</button>
				</div>
			</div>
			<div class="${E.inputSectionWrapper} ${E.hidden}">
				<div class="${E.inputSection}">
					<input 
						type="text" 
						class="${E.taskInput}" 
						maxlength="1000"
					/>
				</div>
			</div>
		`,document.body.appendChild(t),t}#D(){this.wrapper.querySelector(`.${E.header}`).addEventListener("click",e=>{e.target.closest(`.${E.controlButton}`)||this.#T()}),this.#c.addEventListener("click",e=>{e.stopPropagation(),this.#T()}),this.#a.addEventListener("click",e=>{e.stopPropagation(),this.#Z()}),this.#l.addEventListener("keydown",e=>{e.isComposing||e.key==="Enter"&&(e.preventDefault(),this.#R())}),this.#s.addEventListener("click",e=>{e.stopPropagation()})}#T(){this.#h?this.#y():this.#b()}#b(){this.#h=!0,this.wrapper.classList.add(E.expanded),this.#c.textContent="\u25B2"}#y(){this.#h=!1,this.wrapper.classList.remove(E.expanded),this.#c.textContent="\u25BC"}#j(){this.#m=setInterval(()=>{this.#B()},450)}#U(){this.#m&&(clearInterval(this.#m),this.#m=null)}#B(){if(!this.#u||this.#x)return;if(this.#t.textContent===this.#u){this.#u=null;return}let e=this.#u;this.#u=null,this.#W(e)}#W(e){this.#x=!0,this.#t.classList.add(E.fadeOut),setTimeout(()=>{this.#t.textContent=e,this.#t.classList.remove(E.fadeOut),this.#t.classList.add(E.fadeIn),setTimeout(()=>{this.#t.classList.remove(E.fadeIn),this.#x=!1},300)},150)}#g(e){let t=e==="running"?"thinking":e;this.#o.className=E.indicator,t!=="idle"&&t!=="stopped"&&this.#o.classList.add(E[t])}#P(){setTimeout(()=>{this.#i.scrollTop=this.#i.scrollHeight},0)}#A(){let e=[],t=this.#e.task;t&&e.push(this.#V(t));let n=this.#e.history;for(let r of n)e.push(...this.#G(r));this.#i.innerHTML=e.join(""),this.#P()}#V(e){return he({icon:"\u{1F3AF}",content:e,type:"input"})}#G(e){let t=[],n=e.type==="step"&&e.stepIndex!==void 0?this.#r.t("ui.panel.step",{number:(e.stepIndex+1).toString()}):void 0;if(e.type==="step"){if(e.reflection){let o=Zl(e.reflection);o.length>0&&t.push(he({icon:"\u{1F9E0}",content:o,meta:n}))}let r=e.action;r&&t.push(...this.#J(r,n))}else if(e.type==="observation")t.push(he({icon:"\u{1F441}\uFE0F",content:e.content||"",meta:n,type:"observation"}));else if(e.type==="user_takeover")t.push(he({icon:"\u{1F464}",content:"User takeover",meta:n,type:"input"}));else if(e.type==="retry"){let r=`${e.message||"Retrying"} (${e.attempt}/${e.maxAttempts})`;t.push(he({icon:"\u{1F504}",content:r,meta:n,type:"observation"}))}else e.type==="error"&&t.push(he({icon:"\u274C",content:e.message||"Error",meta:n,type:"observation"}));return t}#J(e,t){let n=[];if(e.name==="done"){let r=e.input.text||e.output||"";r&&n.push(he({icon:"\u{1F916}",content:r,meta:t,type:"output"}))}else if(e.name==="ask_user"){let r=e.input,o=e.output.replace(/^User answered:\s*/i,"");n.push(he({icon:"\u2753",content:`Question: ${r.question||""}`,meta:t,type:"question"})),n.push(he({icon:"\u{1F4AC}",content:`Answer: ${o}`,meta:t,type:"input"}))}else{let r=this.#E(e.name,e.input);n.push(he({icon:"\u{1F528}",content:r,meta:t})),e.output?.length>0&&n.push(he({icon:"\u{1F528}",content:e.output,meta:t,type:"output"}))}return n}};var $h=class extends da{panel;constructor(e){let t=new Aa({...e,enableMask:e.enableMask??!0});super({...e,pageController:t}),this.panel=new Ca(this,{language:e.language,promptForNextTask:e.promptForNextTask})}};export{$h as PageAgent,da as PageAgentCore,uh as tool};
/*! Bundled license information:

ai-motion/build/Motion.js:
  (**
   * AI Motion - WebGL2 animated border with AI-style glow effects
   *
   * @author Simon<gaomeng1900@gmail.com>
   * @license MIT
   * @repository https://github.com/gaomeng1900/ai-motion
   *)
*/
