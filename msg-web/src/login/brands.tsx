/**
 * # 七站登录页 · 品牌与业务内容层（E2 光轴穿联定稿）
 *
 * 统一的是**版式与控件**（见 LoginShell / kit.css），**不统一文字**。
 * 页面只渲染四样东西：字标（mark + name）、一句逐站独立的 tagline、
 * 两个输入框 / 登录按钮、以及插画左下角一行极短术语 shortTerms。
 *
 * 插画是裸图形：没有窗体框、没有标题栏，图内最多 4 个短词
 * （2 个英文分区标签 + 2 个中文业务关键词），数值与说明文字全部剥掉。
 *
 * headline / description / features / noteBody 等长文案保留为**资料层**：
 * 逐字取自各站旧登录页或 README，供方案稿与设计说明引用，E2 正式页不渲染。
 *
 * 本文件是唯一真源，由 sync.py 同步到七个前端，不要在项目副本里手改。
 */

import type { ReactNode } from "react";

export type BrandKey = "adm" | "jobx" | "msg" | "alarm" | "dbz" | "dtw" | "syn";

export interface SceneStat {
  value: string;
  label: string;
  sub: string;
}

export interface LoginBrand {
  key: BrandKey;
  /** 字标里的短码 */
  mark: string;
  /** 控制台名 */
  name: string;
  /** 字标下的业务小字 */
  en: string;
  /** 字标下一句话业务标语：页面唯一的一行描述，逐站独立、句式不统一 */
  tagline: string;
  /** 插画左下角一行极短术语，只放领域缩写，不成句 */
  shortTerms: string;
  /** 左栏眉标 */
  eyebrow: string;
  /** 左栏主标题，两行 */
  headline: [string, string];
  /** 左栏业务描述（旧登录页原文 / 该站 README 原文） */
  description: string;
  /** 左栏三条业务要点（旧登录页原文 / 该站领域术语） */
  features: [string, string, string];
  /** 左栏页脚术语条 */
  terms: string;
  /** 左栏底部业务注脚 */
  storyFoot: string;
  /** 右栏表单标题 */
  formTitle: string;
  /** 右栏表单副标题 */
  formSubtitle: string;
  accountPlaceholder: string;
  passwordPlaceholder: string;
  submitLabel: string;
  busyLabel: string;
  /** 右栏说明卡标题 */
  noteTitle: string;
  /** 右栏说明卡正文 */
  noteBody: string;
  /** 场景插画下方的一行图注 */
  sceneCaption: string;
  /** 浮在插画上的两张业务数据卡 */
  stats: [SceneStat, SceneStat];
  /** 业务场景插画 */
  glyph: ReactNode;
}

/** 套件导航：左栏底部七个字标，当前站点高亮 */
export const SUITE: Array<{ key: BrandKey; label: string }> = [
  { key: "adm", label: "后台管理" },
  { key: "jobx", label: "JobX" },
  { key: "msg", label: "消息" },
  { key: "alarm", label: "报警" },
  { key: "dbz", label: "dbz" },
  { key: "dtw", label: "DTW" },
  { key: "syn", label: "Syn" },
];

/* ------------------------------------------------------------------ *
 * 1 / 7  Adm · 组织树 → 角色 → 权限矩阵
 * ------------------------------------------------------------------ */
const admGlyph = (
  <svg viewBox="32 92 616 268" className="g-scene" role="img" aria-label="组织树、角色与权限矩阵">
    <defs>
      <linearGradient id="adm-fill" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stopColor="#8fa8ff" stopOpacity="0.42" />
        <stop offset="1" stopColor="#4957d8" stopOpacity="0.06" />
      </linearGradient>
      <linearGradient id="adm-bar" x1="0" y1="1" x2="0" y2="0">
        <stop offset="0" stopColor="#6d8bff" stopOpacity="0.15" />
        <stop offset="1" stopColor="#a5b6ff" stopOpacity="0.95" />
      </linearGradient>
    </defs>
    <circle cx="556" cy="60" r="128" fill="url(#adm-fill)" opacity="0.55" />
    <circle cx="96" cy="356" r="104" fill="url(#adm-fill)" opacity="0.35" />

    <text x="62" y="110" className="g-label">ORG TREE</text>
    <rect x="62" y="122" width="176" height="38" rx="10" className="g-card-a" />
    <circle cx="82" cy="141" r="11" className="g-a" />
    <text x="100" y="146" className="g-t">集团总部</text>

    <path d="M82 160v18m0 0h46m0 0v18m0 0h46m0 0v18M82 178v0" className="g-line" />
    <path d="M82 178h128v-0" className="g-line" />
    <path d="M82 178v34m0 0h128m-64 0v18" className="g-line" />
    <rect x="104" y="196" width="132" height="32" rx="8" className="g-card" />
    <circle cx="120" cy="212" r="7" className="g-b" />
    <text x="134" y="216" className="g-t2">华东事业部</text>
    <rect x="104" y="234" width="132" height="32" rx="8" className="g-card" />
    <circle cx="120" cy="250" r="7" className="g-b" />

    <rect x="104" y="272" width="132" height="32" rx="8" className="g-card" />
    <circle cx="120" cy="288" r="7" className="g-b" />

    <path d="M174 212h30m-30 38h30m-30 38h30" className="g-line-a" />

    <path d="M252 104v234" className="g-line" />
    <text x="270" y="110" className="g-label">ROLE</text>
    <rect x="270" y="122" width="166" height="34" rx="9" className="g-card-a" />

    <circle cx="416" cy="139" r="6" className="g-a" />
    <rect x="270" y="164" width="166" height="34" rx="9" className="g-card" />

    <rect x="270" y="206" width="166" height="34" rx="9" className="g-card" />

    <rect x="270" y="248" width="166" height="34" rx="9" className="g-card" />

    <rect x="270" y="296" width="106" height="8" rx="4" className="g-track" />
    <rect x="270" y="296" width="72" height="8" rx="4" fill="url(#adm-bar)" />

    <path d="M454 104v234" className="g-line" />

    <g className="g-matrix">
      <rect x="472" y="142" width="150" height="26" rx="6" className="g-card" />
      <rect x="480" y="150" width="10" height="10" rx="2" className="g-a" />
      <rect x="516" y="150" width="10" height="10" rx="2" className="g-a" />
      <rect x="552" y="150" width="10" height="10" rx="2" className="g-a" />
      <rect x="588" y="150" width="10" height="10" rx="2" className="g-a" />
      <rect x="472" y="176" width="150" height="26" rx="6" className="g-card" />
      <rect x="480" y="184" width="10" height="10" rx="2" className="g-a" />
      <rect x="516" y="184" width="10" height="10" rx="2" className="g-a" />
      <rect x="552" y="184" width="10" height="10" rx="2" className="g-hollow" />
      <rect x="588" y="184" width="10" height="10" rx="2" className="g-hollow" />
      <rect x="472" y="210" width="150" height="26" rx="6" className="g-card" />
      <rect x="480" y="218" width="10" height="10" rx="2" className="g-a" />
      <rect x="516" y="218" width="10" height="10" rx="2" className="g-hollow" />
      <rect x="552" y="218" width="10" height="10" rx="2" className="g-hollow" />
      <rect x="588" y="218" width="10" height="10" rx="2" className="g-hollow" />
      <rect x="472" y="244" width="150" height="26" rx="6" className="g-card" />
      <rect x="480" y="252" width="10" height="10" rx="2" className="g-b" />
      <rect x="516" y="252" width="10" height="10" rx="2" className="g-hollow" />
      <rect x="552" y="252" width="10" height="10" rx="2" className="g-hollow" />
      <rect x="588" y="252" width="10" height="10" rx="2" className="g-hollow" />
    </g>
    <rect x="472" y="288" width="150" height="32" rx="8" className="g-card-b" />

    <circle cx="606" cy="304" r="5" className="g-ok" />

    <path d="M236 141h14m166 0h38" className="g-flow" />
    <circle cx="252" cy="141" r="4" className="g-dot" />
    <circle cx="454" cy="141" r="4" className="g-dot" />

  </svg>
);

/* ------------------------------------------------------------------ *
 * 2 / 7  JobX · Cron 时间轴 + 甘特任务条 + 执行器
 * ------------------------------------------------------------------ */
const jobxGlyph = (
  <svg viewBox="32 92 616 268" className="g-scene" role="img" aria-label="调度时间轴、任务条与执行器状态">
    <defs>
      <linearGradient id="jobx-bar" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0" stopColor="#35d3a0" stopOpacity="0.95" />
        <stop offset="1" stopColor="#128f6c" stopOpacity="0.5" />
      </linearGradient>
      <linearGradient id="jobx-area" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stopColor="#4ade9f" stopOpacity="0.42" />
        <stop offset="1" stopColor="#4ade9f" stopOpacity="0" />
      </linearGradient>
    </defs>
    <circle cx="120" cy="76" r="120" fill="url(#jobx-area)" />
    <circle cx="588" cy="348" r="130" fill="url(#jobx-area)" opacity="0.6" />

    <text x="62" y="108" className="g-label">SCHEDULE</text>

    <path d="M96 138v176M214 138v176M332 138v176M450 138v176" className="g-grid-line" />
    <text x="62" y="160" className="g-t2">日报汇总</text>
    <text x="62" y="200" className="g-t2">缓存刷新</text>

    <rect x="96" y="148" width="92" height="16" rx="8" fill="url(#jobx-bar)" />
    <rect x="220" y="148" width="92" height="16" rx="8" fill="url(#jobx-bar)" opacity="0.55" />
    <rect x="344" y="148" width="92" height="16" rx="8" fill="url(#jobx-bar)" opacity="0.35" />
    <rect x="96" y="188" width="42" height="16" rx="8" className="g-b" />
    <rect x="154" y="188" width="42" height="16" rx="8" className="g-b" opacity="0.7" />
    <rect x="212" y="188" width="42" height="16" rx="8" className="g-b" opacity="0.7" />
    <rect x="270" y="188" width="42" height="16" rx="8" className="g-b" opacity="0.7" />
    <rect x="328" y="188" width="42" height="16" rx="8" className="g-b" opacity="0.7" />
    <rect x="386" y="188" width="42" height="16" rx="8" className="g-b" opacity="0.7" />
    <rect x="96" y="228" width="352" height="16" rx="8" fill="url(#jobx-bar)" opacity="0.32" />
    <circle cx="452" cy="236" r="10" className="g-pulse" />
    <rect x="96" y="268" width="126" height="16" rx="8" className="g-c" opacity="0.85" />
    <rect x="238" y="268" width="126" height="16" rx="8" className="g-bad" opacity="0.75" />
    <path d="M96 314h350" className="g-line" />

    <path d="M470 104v234" className="g-line" />
    <text x="488" y="110" className="g-label">EXECUTOR</text>
    <rect x="488" y="122" width="134" height="52" rx="10" className="g-card-a" />

    <rect x="500" y="152" width="110" height="6" rx="3" className="g-track" />
    <rect x="500" y="152" width="68" height="6" rx="3" className="g-a" />

    <rect x="488" y="182" width="134" height="52" rx="10" className="g-card" />

    <rect x="500" y="212" width="110" height="6" rx="3" className="g-track" />
    <rect x="500" y="212" width="41" height="6" rx="3" className="g-b" />

    <rect x="488" y="242" width="134" height="52" rx="10" className="g-card" />

    <rect x="500" y="272" width="110" height="6" rx="3" className="g-track" />
    <rect x="500" y="272" width="88" height="6" rx="3" className="g-c" />

    <rect x="488" y="304" width="134" height="30" rx="9" className="g-card-b" />

  </svg>
);

/* ------------------------------------------------------------------ *
 * 3 / 7  Msg · 事件 → 模板 → 多渠道扇出
 * ------------------------------------------------------------------ */
const msgGlyph = (
  <svg viewBox="32 92 616 268" className="g-scene" role="img" aria-label="消息模板与多渠道投递扇出">
    <defs>
      <linearGradient id="msg-card" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stopColor="#7cc6ff" stopOpacity="0.34" />
        <stop offset="1" stopColor="#2563eb" stopOpacity="0.06" />
      </linearGradient>
      <linearGradient id="msg-area" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stopColor="#4aa8ff" stopOpacity="0.38" />
        <stop offset="1" stopColor="#4aa8ff" stopOpacity="0" />
      </linearGradient>
    </defs>
    <circle cx="560" cy="88" r="140" fill="url(#msg-area)" />
    <circle cx="90" cy="360" r="120" fill="url(#msg-area)" opacity="0.55" />

    <text x="62" y="110" className="g-label">EVENT</text>
    <rect x="62" y="122" width="176" height="196" rx="14" fill="url(#msg-card)" className="g-stroke" />
    <rect x="78" y="138" width="92" height="18" rx="6" className="g-pill" />

    <text x="78" y="180" className="g-t">alarm.</text>
    <path d="M78 194h144" className="g-line" />

    <rect x="78" y="222" width="144" height="12" rx="6" className="g-bar" />
    <rect x="78" y="238" width="104" height="12" rx="6" className="g-bar" opacity="0.6" />

    <rect x="78" y="280" width="128" height="12" rx="6" className="g-bar" />
    <rect x="78" y="296" width="144" height="12" rx="6" className="g-bar" opacity="0.6" />
    <circle cx="214" cy="220" r="20" className="g-a" opacity="0.28" />
    <text x="214" y="225" className="g-t" textAnchor="middle">T</text>

    <path d="M244 160c34 0 34-46 68-46M244 220h68M244 280c34 0 34 46 68 46" className="g-flow" />
    <circle cx="272" cy="141" r="4" className="g-dot" />
    <circle cx="278" cy="220" r="4" className="g-dot" />
    <circle cx="272" cy="299" r="4" className="g-dot" />

    <text x="324" y="110" className="g-label">CHANNEL BINDIN</text>
    <rect x="324" y="114" width="164" height="58" rx="12" className="g-card-a" />
    <circle cx="346" cy="137" r="12" className="g-a" />

    <rect x="324" y="184" width="164" height="58" rx="12" className="g-card" />
    <circle cx="346" cy="207" r="12" className="g-b" />

    <rect x="324" y="254" width="164" height="58" rx="12" className="g-card" />
    <circle cx="346" cy="277" r="12" className="g-c" />

    <path d="M506 104v234" className="g-line" />

    <path d="M524 152h98" className="g-line" />
    <rect x="524" y="166" width="98" height="9" rx="4" className="g-track" />
    <rect x="524" y="166" width="92" height="9" rx="4" className="g-a" />

    <rect x="524" y="202" width="98" height="9" rx="4" className="g-track" />
    <rect x="524" y="202" width="7" height="9" rx="4" className="g-bad" />

    <rect x="524" y="238" width="98" height="9" rx="4" className="g-track" />
    <rect x="524" y="238" width="4" height="9" rx="4" className="g-warn" />

    <rect x="524" y="278" width="98" height="34" rx="9" className="g-card-b" />

  </svg>
);

/* ------------------------------------------------------------------ *
 * 4 / 7  Alarm · 越阈值波形 + P1–P4 分级
 * ------------------------------------------------------------------ */
const alarmGlyph = (
  <svg viewBox="32 92 616 268" className="g-scene" role="img" aria-label="报警波形越过阈值并按等级分流">
    <defs>
      <linearGradient id="alarm-wave" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stopColor="#ff9c5b" stopOpacity="0.72" />
        <stop offset="1" stopColor="#ff9c5b" stopOpacity="0.02" />
      </linearGradient>
      <linearGradient id="alarm-hot" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0" stopColor="#ff5f3d" stopOpacity="0.1" />
        <stop offset="1" stopColor="#ff5f3d" stopOpacity="0.34" />
      </linearGradient>
    </defs>
    <circle cx="574" cy="60" r="140" fill="url(#alarm-hot)" />
    <circle cx="70" cy="370" r="110" fill="url(#alarm-hot)" opacity="0.5" />

    <text x="62" y="110" className="g-label">EVENT STREAM</text>
    <rect x="62" y="122" width="372" height="196" rx="12" fill="url(#alarm-hot)" className="g-stroke" />
    <path d="M62 168h372M62 214h372M62 260h372" className="g-grid-line" />
    <path d="M62 146h372" className="g-threshold" />

    <path d="M62 278c26 0 34-8 54-8s28 10 46 10 26-24 44-24 22 20 40 20 26-46 46-46 20 30 40 30 24-96 44-96 22 52 58 52v116H62Z" fill="url(#alarm-wave)" />
    <path d="M62 278c26 0 34-8 54-8s28 10 46 10 26-24 44-24 22 20 40 20 26-46 46-46 20 30 40 30 24-96 44-96 22 52 58 52" className="g-wave" />
    <circle cx="390" cy="130" r="9" className="g-bad" />
    <circle cx="390" cy="130" r="17" className="g-pulse" />

    <path d="M452 104v234" className="g-line" />
    <text x="470" y="110" className="g-label">SEVERITY</text>
    <rect x="470" y="122" width="152" height="38" rx="9" className="g-sev-p1" />
    <rect x="470" y="122" width="5" height="38" rx="2" className="g-bad" />
    <text x="486" y="147" className="g-t">P1 紧急</text>
    <text x="604" y="147" className="g-t" textAnchor="end">2</text>
    <rect x="470" y="168" width="152" height="38" rx="9" className="g-card" />
    <rect x="470" y="168" width="5" height="38" rx="2" className="g-warn" />

    <rect x="470" y="214" width="152" height="38" rx="9" className="g-card" />
    <rect x="470" y="214" width="5" height="38" rx="2" className="g-c" />

    <rect x="470" y="260" width="152" height="38" rx="9" className="g-card" />
    <rect x="470" y="260" width="5" height="38" rx="2" className="g-b" />

    <rect x="470" y="306" width="152" height="30" rx="9" className="g-card-b" />

  </svg>
);

/* ------------------------------------------------------------------ *
 * 5 / 7  dbz · 源库 → Connector → topic/offset → Sink
 * ------------------------------------------------------------------ */
const dbzGlyph = (
  <svg viewBox="32 92 616 268" className="g-scene" role="img" aria-label="变更捕获管道：源库、连接器、Kafka topic 与下游">
    <defs>
      <linearGradient id="dbz-cyl" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0" stopColor="#a78bfa" stopOpacity="0.25" />
        <stop offset="0.5" stopColor="#c4b5fd" stopOpacity="0.62" />
        <stop offset="1" stopColor="#7c3aed" stopOpacity="0.2" />
      </linearGradient>
      <linearGradient id="dbz-lane" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0" stopColor="#a78bfa" stopOpacity="0.85" />
        <stop offset="1" stopColor="#f0abfc" stopOpacity="0.2" />
      </linearGradient>
    </defs>
    <circle cx="570" cy="72" r="132" fill="url(#dbz-lane)" opacity="0.28" />
    <circle cx="86" cy="356" r="112" fill="url(#dbz-lane)" opacity="0.2" />

    <text x="62" y="110" className="g-label">SOURCE</text>
    <path d="M62 146c0-10 26-18 58-18s58 8 58 18v72c0 10-26 18-58 18s-58-8-58-18Z" fill="url(#dbz-cyl)" className="g-stroke" />
    <ellipse cx="120" cy="146" rx="58" ry="18" fill="url(#dbz-cyl)" className="g-stroke" />
    <path d="M62 182c0 10 26 18 58 18s58-8 58-18M62 218c0 10 26 18 58 18s58-8 58-18" className="g-line" />
    <text x="120" y="152" className="g-t" textAnchor="middle">MySQL</text>

    <path d="M178 182h56" className="g-flow" />
    <circle cx="204" cy="182" r="4" className="g-dot" />
    <rect x="234" y="140" width="118" height="84" rx="12" className="g-card-a" />
    <text x="248" y="162" className="g-t">Debezi</text>

    <rect x="248" y="192" width="90" height="7" rx="3" className="g-track" />
    <rect x="248" y="192" width="76" height="7" rx="3" className="g-a" />

    <path d="M352 182h44" className="g-flow" />
    <circle cx="374" cy="182" r="4" className="g-dot" />

    <text x="396" y="110" className="g-label">OFFSET</text>
    <rect x="396" y="122" width="176" height="40" rx="9" className="g-card" />
    <rect x="408" y="134" width="88" height="14" rx="7" fill="url(#dbz-lane)" />

    <rect x="396" y="170" width="176" height="40" rx="9" className="g-card" />
    <rect x="408" y="182" width="120" height="14" rx="7" fill="url(#dbz-lane)" opacity="0.75" />

    <rect x="396" y="218" width="176" height="40" rx="9" className="g-card" />
    <rect x="408" y="230" width="66" height="14" rx="7" fill="url(#dbz-lane)" opacity="0.5" />

    <path d="M396 276h176" className="g-line" />

    <rect x="396" y="306" width="176" height="30" rx="9" className="g-card-b" />

    <rect x="62" y="320" width="74" height="24" rx="7" className="g-card" />

    <rect x="144" y="320" width="74" height="24" rx="7" className="g-card" />

    <rect x="226" y="320" width="74" height="24" rx="7" className="g-card" />

    <path d="M352 224v96H120" className="g-flow" />
    <circle cx="352" cy="272" r="4" className="g-dot" />

  </svg>
);

/* ------------------------------------------------------------------ *
 * 6 / 7  DTW · WKT 多边形围栏 + 定位点 + 半径圈
 * ------------------------------------------------------------------ */
const dtwGlyph = (
  <svg viewBox="32 92 616 268" className="g-scene" role="img" aria-label="WKT 多边形电子围栏、定位轨迹与半径圈">
    <defs>
      <linearGradient id="dtw-fence" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stopColor="#3ee0d0" stopOpacity="0.46" />
        <stop offset="1" stopColor="#0e7490" stopOpacity="0.08" />
      </linearGradient>
      <radialGradient id="dtw-glow">
        <stop offset="0" stopColor="#5eead4" stopOpacity="0.5" />
        <stop offset="1" stopColor="#5eead4" stopOpacity="0" />
      </radialGradient>
      <pattern id="dtw-grid" width="28" height="28" patternUnits="userSpaceOnUse">
        <path d="M28 0H0v28" className="g-grid-line" />
      </pattern>
    </defs>

    <path d="M40 168h160l72-52 96 42 272-30M40 272l110-40 88 44 96-26 306 32" className="g-road" />
    <path d="M212 84v274M402 84v274" className="g-road" opacity="0.5" />
    <path d="M96 132 196 118 268 156 318 140 386 190 352 268 262 292 158 262 108 202Z" fill="url(#dtw-fence)" className="g-fence" />
    <path d="M96 132 196 118 268 156 318 140 386 190 352 268 262 292 158 262 108 202Z" className="g-fence-dash" />

    <path d="M412 168 486 138 566 166 596 232 536 276 446 258 402 210Z" fill="url(#dtw-fence)" opacity="0.65" className="g-fence" />

    <circle cx="228" cy="206" r="62" fill="url(#dtw-glow)" />
    <circle cx="228" cy="206" r="46" className="g-ring" />
    <circle cx="228" cy="206" r="30" className="g-ring" opacity="0.6" />
    <circle cx="228" cy="206" r="10" className="g-a" />
    <circle cx="228" cy="206" r="3" className="g-core" />
    <circle cx="176" cy="232" r="6" className="g-b" />
    <circle cx="292" cy="246" r="6" className="g-b" />
    <circle cx="316" cy="184" r="6" className="g-b" />
    <circle cx="478" cy="212" r="6" className="g-c" />
    <circle cx="528" cy="238" r="6" className="g-c" />
    <path d="M176 232c22 8 30-18 52-26s48 12 64 40" className="g-trajectory" />
    <circle cx="292" cy="246" r="12" className="g-pulse" />

    <rect x="470" y="292" width="152" height="52" rx="12" className="g-card-a" />
    <text x="484" y="314" className="g-t">在区 148</text>

    <rect x="58" y="292" width="152" height="52" rx="12" className="g-card-b" />
    <text x="72" y="314" className="g-t2">规则快照</text>

  </svg>
);

/* ------------------------------------------------------------------ *
 * 7 / 7  Syn · 五段编排流水线 + 幂等台账
 * ------------------------------------------------------------------ */
const synGlyph = (
  <svg viewBox="32 92 616 268" className="g-scene" role="img" aria-label="五段同步编排流水线与幂等回放">
    <defs>
      <linearGradient id="syn-stage" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stopColor="#fbcf6a" stopOpacity="0.38" />
        <stop offset="1" stopColor="#b45309" stopOpacity="0.06" />
      </linearGradient>
      <linearGradient id="syn-line" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0" stopColor="#fbbf24" stopOpacity="0.95" />
        <stop offset="1" stopColor="#f59e0b" stopOpacity="0.35" />
      </linearGradient>
    </defs>
    <circle cx="574" cy="66" r="136" fill="url(#syn-stage)" />
    <circle cx="78" cy="366" r="112" fill="url(#syn-stage)" opacity="0.6" />

    <text x="62" y="110" className="g-label">FIVE STAGES</text>
    <rect x="62" y="122" width="106" height="90" rx="12" fill="url(#syn-stage)" className="g-stroke" />
    <text x="76" y="146" className="g-t">01 匹配</text>

    <rect x="76" y="192" width="78" height="7" rx="3" className="g-track" />
    <rect x="76" y="192" width="62" height="7" rx="3" className="g-a" />
    <rect x="180" y="122" width="106" height="90" rx="12" className="g-card" />
    <text x="194" y="146" className="g-t2">02 映射</text>

    <rect x="194" y="192" width="78" height="7" rx="3" className="g-track" />
    <rect x="194" y="192" width="48" height="7" rx="3" className="g-b" />
    <rect x="298" y="122" width="106" height="90" rx="12" className="g-card" />

    <rect x="312" y="192" width="78" height="7" rx="3" className="g-track" />
    <rect x="312" y="192" width="34" height="7" rx="3" className="g-c" />
    <rect x="416" y="122" width="106" height="90" rx="12" className="g-card" />

    <rect x="430" y="192" width="78" height="7" rx="3" className="g-track" />
    <rect x="430" y="192" width="58" height="7" rx="3" className="g-a" />
    <rect x="534" y="122" width="88" height="90" rx="12" className="g-card-a" />

    <rect x="548" y="192" width="60" height="7" rx="3" className="g-track" />
    <rect x="548" y="192" width="44" height="7" rx="3" className="g-warn" />

    <path d="M168 167h12M286 167h12M404 167h12M522 167h12" className="g-flow" />
    <circle cx="174" cy="167" r="3.5" className="g-dot" />
    <circle cx="292" cy="167" r="3.5" className="g-dot" />
    <circle cx="410" cy="167" r="3.5" className="g-dot" />
    <circle cx="528" cy="167" r="3.5" className="g-dot" />

    <text x="62" y="242" className="g-label">VERDICT</text>
    <rect x="62" y="254" width="132" height="38" rx="9" className="g-card" />
    <circle cx="80" cy="273" r="6" className="g-ok" />

    <rect x="204" y="254" width="132" height="38" rx="9" className="g-card" />
    <circle cx="222" cy="273" r="6" className="g-b" />

    <rect x="346" y="254" width="132" height="38" rx="9" className="g-card" />
    <circle cx="364" cy="273" r="6" className="g-bad" />

    <rect x="488" y="254" width="134" height="38" rx="9" className="g-card" />
    <circle cx="506" cy="273" r="6" className="g-warn" />

    <path d="M62 312h560" className="g-line" />
    <rect x="62" y="320" width="270" height="26" rx="8" className="g-card-b" />

    <path d="M622 333h-236" className="g-flow" />
    <circle cx="470" cy="333" r="4" className="g-dot" />

  </svg>
);

/* ------------------------------------------------------------------ *
 * 品牌配置 · 每站文案独立，不做统一默认
 * ------------------------------------------------------------------ */
export const BRANDS: Record<BrandKey, LoginBrand> = {
  adm: {
    tagline: "账号、角色、菜单与可见范围，集中授权。",
    shortTerms: "账号 · 角色 · 菜单 · 范围",
    key: "adm",
    mark: "AD",
    name: "后台管理",
    en: "账号 · 角色 · 菜单 · 权限",
    eyebrow: "统一身份与权限",
    headline: ["把账号、角色与权限", "放在同一张桌面上。"],
    description: "账号、角色、菜单与授权关系在这里集中维护。登录后只会看到你有权限使用的页面，权限调整后立即生效。",
    features: ["口令加密后传输，网络上不会出现明文", "菜单与功能按你的角色自动呈现", "权限改动保存后立即生效，无需重启系统"],
    terms: "账号 · 角色 · 领域 / 组织 / 应用范围",
    storyFoot: "界面只展示你有权限使用的功能，所有操作都以你当前的授权为准。",
    formTitle: "登录管理后台",
    formSubtitle: "用你的登录名、手机号、邮箱或身份证号登录。",
    accountPlaceholder: "登录名 / 手机号 / 邮箱 / 身份证号",
    passwordPlaceholder: "请输入口令",
    submitLabel: "登录",
    busyLabel: "正在校验…",
    noteTitle: "首次登录",
    noteBody: "初始账号由管理员开通。登录后能看到哪些菜单、能用哪些功能，取决于你的角色；如果登录后没有可用菜单，请联系管理员。首次登录请到个人中心修改口令，不要长期使用默认口令。",
    sceneCaption: "组织树 → 角色 → 权限矩阵，授权即时生效",
    stats: [{ value: "按角色", label: "可见菜单", sub: "登录后自动呈现" }, { value: "三层", label: "可见范围", sub: "领域 / 组织 / 应用" }],
    glyph: admGlyph,
  },
  jobx: {
    tagline: "定时计划与执行器状态，一屏对照。",
    shortTerms: "计划 · 执行 · 记录",
    key: "jobx",
    mark: "JX",
    name: "JobX 控制台",
    en: "计划 · 分组 · 执行记录",
    eyebrow: "任务调度",
    headline: ["哪些计划还活着，", "下一次分派在几秒后。"],
    description: "在这里安排定时任务、管理分组，查看每次执行的结果。登录状态在七个控制台之间通用，退出一次全部退出。",
    features: ["按时间表达式定时执行，也支持固定频率和固定间隔", "计划、分组、执行器与执行记录同屏对照", "失败原因直接写在记录里，值班时不用翻日志"],
    terms: "定时 · 固定频率 · 固定间隔 · 手工触发",
    storyFoot: "给值班的人看：计划是否正常、下一次什么时候跑、上一次为什么失败，一屏就能看出来。",
    formTitle: "登录",
    formSubtitle: "使用统一账号登录",
    accountPlaceholder: "登录账号",
    passwordPlaceholder: "登录密码",
    submitLabel: "登录",
    busyLabel: "登录中…",
    noteTitle: "统一登录态",
    noteBody: "JobX 不单独维护账号，用统一账号登录即可。登录状态在七个控制台之间通用，退出一次全部失效。",
    sceneCaption: "定时计划 × 执行状态",
    stats: [{ value: "自动", label: "下一次执行", sub: "按计划自动排定" }, { value: "逐条", label: "执行结果", sub: "成功 / 失败 / 跳过" }],
    glyph: jobxGlyph,
  },
  msg: {
    tagline: "一条事件，按模板扇出到各条渠道。",
    shortTerms: "事件 · 模板 · 渠道",
    key: "msg",
    mark: "消",
    name: "消息中心",
    en: "消息中心",
    eyebrow: "消息定义与投递",
    headline: ["一条事件，", "按模板扇出到每条渠道。"],
    description: "一个事件编码对应一条消息定义：模板决定文案，目标决定发给谁，渠道决定怎么发。",
    features: ["消息定义、来源、分类与订阅主题集中维护", "标题和内容按模板生成，目标类别自动对应渠道", "投递记录、失败重投与死信处理在同一条链路上完成"],
    terms: "事件 · 模板 · 目标 · 渠道 · 失败重投",
    storyFoot: "每次发送都会留一条记录：接收 → 发送 → 完成，失败原因写在详情里。",
    formTitle: "登录消息中心",
    formSubtitle: "登录后管理消息定义与发送渠道",
    accountPlaceholder: "账号",
    passwordPlaceholder: "密码",
    submitLabel: "登录",
    busyLabel: "登录中…",
    noteTitle: "投递链路",
    noteBody: "消息中心与统一账号共用登录。业务方把事件发到对应主题，系统按订阅展开成具体目标和渠道，再逐条投递。",
    sceneCaption: "事件 → 模板 → 目标 → 渠道 → 发送记录",
    stats: [{ value: "逐条", label: "发送记录", sub: "成功 / 失败可查" }, { value: "按类", label: "目标与渠道", sub: "模板自动对应" }],
    glyph: msgGlyph,
  },
  alarm: {
    tagline: "越过阈值即分级，通知按策略自动展开。",
    shortTerms: "事件 · 策略 · 通知",
    key: "alarm",
    mark: "警",
    name: "报警中心",
    en: "报警中心",
    eyebrow: "报警与通知",
    headline: ["事件越过阈值那一刻，", "等级与通知链路就定好了。"],
    description: "事件越过阈值就按策略定级，并自动展开通知。级别、发送队列、合并窗口、防抖和模板都在策略里配置。",
    features: ["报警总量、今日新增与通知送达情况", "按状态处理：确认、忽略、处理完成、手工恢复", "效果字典：火焰 / 烟雾 / 告警点 / 设备损坏"],
    terms: "级别 · 合并窗口 · 防抖 · 手工恢复",
    storyFoot: "同一条事件重复上报只算一次：按事件编码、业务编号和发生时间判重。",
    formTitle: "登录报警中心",
    formSubtitle: "登录后处理报警与配置通知",
    accountPlaceholder: "账号",
    passwordPlaceholder: "密码",
    submitLabel: "登录",
    busyLabel: "登录中…",
    noteTitle: "报警链路",
    noteBody: "报警中心与统一账号共用登录。策略决定级别、发送队列、合并窗口和模板；每条通知的发送结果和失败原因都会记到消息中心，方便追查。",
    sceneCaption: "超过阈值 → 定级 → 合并防抖 → 通知",
    stats: [{ value: "按级", label: "待确认报警", sub: "按严重程度排序" }, { value: "逐条", label: "通知结果", sub: "成功 / 失败可查" }],
    glyph: alarmGlyph,
  },
  dbz: {
    tagline: "从源库 binlog 到下游落地，连接器全程可运维。",
    shortTerms: "变更 · 主题 · 位点",
    key: "dbz",
    mark: "dbz",
    name: "Debezium 控制台",
    en: "数据变更同步",
    eyebrow: "数据变更同步",
    headline: ["从源库变更，", "到下游落地，全程可运维。"],
    description: "管理数据同步集群与上游、下游连接器，并能对线上真正运行的同步任务做运维。",
    features: ["按需对指定表重做一次快照，补齐历史数据", "连接器与主题名必须对应，配错不会报错，只会收不到数据", "失败报文进死信：看原因、重投、标记处理或废弃"],
    terms: "上游 · 连接器 · 主题 · 位点 · 下游 · 死信",
    storyFoot: "下游处理失败的报文会进死信：可以看原因、重投、标记处理或废弃。",
    formTitle: "登录控制台",
    formSubtitle: "使用统一账号登录。",
    accountPlaceholder: "登录名 / 手机号 / 邮箱 / 身份证号",
    passwordPlaceholder: "请输入口令",
    submitLabel: "登录",
    busyLabel: "登录中…",
    noteTitle: "高风险操作",
    noteBody: "登录状态同时用于删除等高风险操作的再次确认。登录过期会自动回到这里，重新登录后可以接着做。",
    sceneCaption: "源库 → 连接器 → 主题与位点 → 下游",
    stats: [{ value: "实时", label: "复制延迟", sub: "持续监控" }, { value: "逐条", label: "失败报文", sub: "死信可重投" }],
    glyph: dbzGlyph,
  },
  dtw: {
    tagline: "WKT 围栏、区域规则与人员定位，同屏对照。",
    shortTerms: "区域 · 围栏 · 规则",
    key: "dtw",
    mark: "DT",
    name: "DTW 人员定位",
    en: "区域控制台",
    eyebrow: "区域 · 围栏 · 报警规则",
    headline: ["区域、规则与报警的", "一体化控制台"],
    description: "用区域树组织空间配置，按坐标围栏维护区域形状；报警规则支持同一区域多档；人员、定位卡和报警记录同屏对照。",
    features: ["三层区域树，支持按全名检索", "登录状态与其他控制台通用", "写操作会记录操作人，便于事后核对"],
    terms: "区域围栏 · 区域树 · 报警规则",
    storyFoot: "登录使用统一账号；如果提示无法连接，请先确认服务是否已启动。",
    formTitle: "登录后台",
    formSubtitle: "使用统一账号登录。",
    accountPlaceholder: "请输入账号",
    passwordPlaceholder: "请输入口令",
    submitLabel: "登录",
    busyLabel: "登录中…",
    noteTitle: "区域与规则",
    noteBody: "静止、聚集的判定阈值在「报警参数」里统一设置，区域规则只负责启停；禁入、禁出、滞留按时段分档。写操作会记录操作人，便于事后核对。",
    sceneCaption: "区域围栏 × 定位点 × 判定半径",
    stats: [{ value: "分层", label: "区域树", sub: "按上级逐级归属" }, { value: "按时段", label: "规则档位", sub: "禁入 / 禁出 / 滞留" }],
    glyph: dtwGlyph,
  },
  syn: {
    tagline: "五段编排先试跑，再落到幂等台账。",
    shortTerms: "匹配 → 判定 · 去重",
    key: "syn",
    mark: "SY",
    name: "Syn 同步台",
    en: "同步控制台",
    eyebrow: "数据同步 · 五段编排",
    headline: ["五段顺序固定，", "改库之前先给出答案。"],
    description: "任务规则、数据抽取、试跑、投递记录与去重台账。图形化配置最怕保存后才发现规则写错，所以试跑要在真正写库之前给出结论。",
    features: ["匹配 → 映射 → 校验 → 投递 → 动作 → 判定，命中即停", "转换支持二进制几何转文本，PostGIS 的十六进制坐标也能走通", "成功 / 跳过 / 失败 / 重试四种结论，决定进度与是否进死信"],
    terms: "匹配 · 映射 · 校验 · 动作 · 判定 · 去重",
    storyFoot: "勾选「真实投递」后按真实链路执行，返回的是下游实际收到的结论。",
    formTitle: "登录同步台",
    formSubtitle: "使用统一账号登录；登录状态在七个控制台之间通用。",
    accountPlaceholder: "登录名 / 手机号 / 邮箱 / 身份证号",
    passwordPlaceholder: "请输入口令",
    submitLabel: "登录",
    busyLabel: "正在校验…",
    noteTitle: "试跑与真实投递",
    noteBody: "试跑只在本地按规则预演，不发送真实请求；勾选「真实投递」才真正执行。登录过期会自动回到登录页，重新登录后接着做。",
    sceneCaption: "数据抽取 → 五段编排 → 投递 → 去重台账",
    stats: [{ value: "五段", label: "规则编排", sub: "命中即停 / 可扇出" }, { value: "逐条", label: "去重记录", sub: "可回放" }],
    glyph: synGlyph,
  },
};
