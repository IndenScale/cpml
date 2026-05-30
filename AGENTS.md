# CPML — Agent 指引

## 项目定位

CPML（Construction Process Modeling Language，施工过程建模语言）是用 Rust 编写的声明式 DSL 编译器。它解析 YAML `.cpml` 文件（建模施工过程中的作业、资源、空间、时间与逻辑约束），执行静态分析以求解可行区间、暴露作业冲突、检测资源竞争。

CPML 是 **CES 产品堆栈的第一层**（核心算法），输出诊断结果供下游系统（数字孪生、WorkSpace、CES）消费。

## 构建与测试

```bash
cargo build                    # debug 构建
cargo build --release          # release 构建（thin LTO, 单 codegen unit）
cargo test                     # 运行全部测试
cargo clippy -- -D warnings    # lint（warning 视为 error）
cargo fmt --check              # 格式检查
```

或使用 `just` 快捷命令：`just check`（clippy + test + fmt）、`just lint`、`just test`、`just fmt`、`just ci`（完整本地 CI 流水线）。

## 架构

```text
src/
├── main.rs          # CLI 入口（clap）：`cpml check` 和 `cpml parse`
├── lib.rs           # 库根文件，重导出 error + 公开模块
├── error.rs         # CpmlError 枚举（thiserror），7 个变体
├── schema/          # YAML 反序列化类型（serde）
│   ├── document.rs  #   CpmlDocument、FieldDef、BarrierDef、FieldType、DiagnosticLevel
│   ├── activity.rs  #   ActivityDef、TimespanDef（字符串日期）
│   ├── geometry.rs  #   GeometryDef、ShapeDef（AABB/Cuboid/Cylinder/Sphere/Hemisphere/Cone）、Pose
│   ├── probe.rs     #   ProbeDef、ProbeCondition（Gte/Lte/Range/Present/Absent）
│   ├── projection.rs#   ProjectionDef
│   └── structure.rs #   CollisionDef、StructureDef（语法糖）
├── model/           # 解析并校验后的领域类型（resolve 阶段之后）
│   ├── activity.rs  #   Activity、Timespan（NaiveDate）、CpmlModel
│   ├── field.rs     #   Field 枚举（Occupancy/Capacity/Scalar/Presence/Rate）
│   ├── geometry.rs  #   Geometry、Shape、Pose（世界空间坐标，基于 parry3d）
│   ├── probe.rs     #   Probe（含已解析的 region key）
│   └── projection.rs#   Projection（含可选的 confidence 评分）
├── pipeline/        # 五阶段编译器流水线
│   ├── orchestrator.rs  # run_pipeline() — 串联各阶段
│   ├── parse.rs     #   阶段 1：YAML → CpmlDocument
│   ├── resolve.rs   #   阶段 2+3：校验 + 构建 CpmlModel
│   ├── expand.rs    #   阶段 4：展开 collision/structure 语法糖 → 显式 probe + projection
│   ├── keyframe.rs  #   阶段 5a：从 activity timespan 中提取去重排序的关键帧日期
│   ├── field_eval.rs#   阶段 5b：逐关键帧计算场状态，更新持久状态
│   └── probe_check.rs  阶段 5c：检查探针，产出含 Blame 归因的 Diagnostic
└── output/
    └── console.rs   # 人类可读的诊断输出（print_result）
```

## 流水线流程

1. **Parse** — 反序列化 YAML 字符串 → `CpmlDocument`
2. **Resolve** — 校验 ID/引用、将字符串日期转为 `NaiveDate`、构建 `CpmlModel`
3. **Expand** — 将 `collision`/`structure` 语法糖展开为显式的 probe + projection；解析几何引用；对无效配置报错（如 structure + presence 未声明 `assert`）
4. **Keyframe** — 从所有 activity 的 start/end 中提取所有唯一日期 → 排序后的关键帧列表
5. **Field Evaluation** — 逐关键帧：找出活跃 activity，以当前场状态检查所有 probe，产出含 Blame 归因的诊断，随后为下一帧更新持久场状态

## 关键设计决策

- **按需计算**：场强仅在探针采样区域内计算，不做全局仿真
- **三阶段碰撞检测**：AABB 快速剔除 → GJK 精确测试（parry3d）→ 遮挡射线投射（仅对软碰撞生效）
- **自身排除**：activity 自身的 projection 不会触发自己的 probe
- **持久状态空间化**（Scalar、Presence、Rate 场）：`field_name → region_key → value`，非全局单值，避免跨区域假阳性
- **Blame 归因**：每个诊断追溯场值来源，列出贡献 projection 及其比例
- **RateField**：基于 `window_size` 的滑动窗口速率计算；`Confidence`（0.0–1.0）对 projection 贡献加权

## 文件类型

| 扩展名   | 用途 |
|----------|------|
| `.cpml`  | YAML 施工过程模型（输入文件） |
| `.rs`    | Rust 源代码 |
| `.toml`  | Cargo 配置、deny 配置、release 配置 |

## 编码规范

- `cargo fmt` 标准 Rust 风格；4 空格缩进、LF 换行（见 `.editorconfig`）
- Clippy warning 即 error（`-D warnings`）
- 不留死代码——直接删除，不注释掉
- 错误类型使用 `thiserror` derive；新增变体统一加在 `src/error.rs`
- 新增字段/特性按顺序推进：先加 schema 类型，再加 model 类型，最后加 pipeline 逻辑

## 文档

- `docs/tutorial.md` — 开发者渐进教程（6 章：施工仿真概念 → CPML 基础 → 编译器 → 场/探针/投影 → 语法糖 → 高级建模）
- `docs/adr/` — 架构决策记录（11 份 ADR）
- `docs/workflow.md` — 流水线流程图与关键设计决策
- `docs/examples.md` — 使用示例
- `docs/TECH_DEBT.md` — 技术债追踪

## 示例

`samples/*.cpml` — 参考输入文件，覆盖碰撞检测、遮挡剔除、许可在场、速率场、资源竞争、标量递进等场景。同时被集成测试与基准测试引用。
