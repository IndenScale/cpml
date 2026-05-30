# CPML 技术债追踪 (Technical Debt Tracker)

本文档追踪在架构决策记录 (ADR) 落地及系统实现过程中遗留的技术债，便于后续迭代规划与管理。

> **最近更新**: 2026-05-10 (v0.2.1 — AIDC 柴发小楼 POC 验证 + 2 项修复)
> **当前状态**: 架构级 4/4 已修复，代码级 5/5 已修复，工程化 10/10 已修复，POC 关键差距 7/7 已修复，POC 验证发现 1/1 已修复，设计改进 2 项待定

## 架构级技术债

### 1. [已修复] 几何与碰撞精确度 (Geometry & Collision Precision)

- **来源**: `adr-geometry.md`
- **问题描述**: ~~当前 `Geometry::world_aabb` 采用的是**保守包围盒估计**。系统对所有旋转的非轴对齐形状计算其世界空间 AABB，并仅依赖 AABB 进行空间重叠检测。~~
- **修复记录**: 已于 2026-05-09 引入 `parry3d-f64` 碰撞库，实现 GJK (Gilbert-Johnson-Keerthi) 精确相交测试。`eval_occupancy` 采用三阶段碰撞检测：AABB 快速剔除 → GJK 精确测试 → 遮挡剔除。新增 `Geometry::exact_intersects()` 方法和 `Shape::to_parry_shape()` 转换。Projection/Probe 现在存储完整 Geometry（而非仅 AABB）以保留精确形状信息。
- **状态**: 已修复

### 2. [已修复] 占用场的遮挡剔除 (Occupancy Occlusion Culling)

- **来源**: `adr-fields.md`
- **问题描述**: ~~根据规范设计，场投影应当支持遮挡剔除（如楼板物理遮挡塔吊的软碰撞风险区）。但目前 `pipeline/field_eval.rs` 尚未实现 Raycasting 或 Shadow-volume 等遮挡剔除规范。~~
- **修复记录**: 已于 2026-05-09 实现基于射线投射的遮挡剔除。在 `CpmlDocument` 中新增 `barriers` 配置段；`eval_occupancy` 中添加 `is_occluded()` 函数，从投影中心向探针中心投射射线检测屏障几何体交集。仅对 Soft 碰撞生效（Hard 碰撞不受遮挡影响）。新增 `samples/occlusion_demo.cpml` 样例及对应集成测试。
- **状态**: 已修复

### 3. [已修复] Structure 语法糖对 Presence 场的无效推断拦截

- **来源**: `adr-structure-expansion.md`
- **问题描述**: ~~`expand.rs` 在自动展开 `structure` 语法糖时，如果没有显式声明 `assert`，会为 `PresenceField` 默认生成一个”空 Key”断言条件。~~
- **修复记录**: 已于 2026-05-09 重构 `expand.rs`，当 Structure 作用于 PresenceField 且未显式声明 `assert` 时，立即抛出 `CpmlError::ValidationError`，提示用户必须提供显式断言（如 `present: {key: “...”}`）。
- **状态**: 已修复

### 4. [已修复] 高级场特性的缺失 (Advanced Field Features)

- **来源**: `adr-fields.md`
- **问题描述**:
  - ~~**连续速率场 (RateField)**: 用于建模流水线流控中的背压（Backpressure）和饥饿（Starvation）状态，目前暂未实现。~~
  - ~~**投影置信度 (Confidence Score)**: 针对外部（如分包商自报）数据的置信度权重体系，尚未引入。~~
- **修复记录**: 已于 2026-05-09 实现：(1) RateField 完整类型，支持 `window_size` 配置的滑动窗口速率计算，新增 `eval_rate()`、`update_rate_state()` 函数，`PersistentState` 扩展 rate 历史存储，支持 Gte 断言；(2) Confidence Score 作为 `Projection` 的可选 `confidence` 字段（0.0-1.0），自动加权 Capacity/Scalar/Rate 贡献值，并在 BlameEntry 和诊断输出中显示。新增 `samples/ratefield_demo.cpml` 样例及对应集成测试。
- **状态**: 已修复

---

## 代码级技术债

### 1. [已修复] 空间探针的全局归因诊断错误

- **发生位置**: `pipeline/probe_check.rs`
- **问题描述**: 之前系统在构建值域场（Value）和存在场（Presence）的失败归因（Blame List）时，仅匹配了 `field_name`，未进行几何体 `region_key` 过滤，导致报出“全局连带责任”的诊断。
- **修复记录**: 已于 2026-05-09 重构 `build_value_blame` 与 `build_presence_blame` 函数，补充完整了 AABB 重叠检测，问题已闭环。

### 2. [已修复] 持久场非空间化存储

- **发生位置**: `pipeline/field_eval.rs`
- **问题描述**: ~~Scalar 和 Presence 持久状态曾退化为全局单值，不同区域的场数据互相污染。~~
- **修复记录**: 已于 2026-05-09 重构为空间化存储（`field_name → region_key → value`），与 Capacity/Occupancy 场保持一致的按区域查询语义，消除了跨区域假阳性。

### 3. [已修复] 死代码清理 + 模块重命名

- **发生位置**: 多个文件
- **修复记录**: 已于 2026-05-09 移除未使用的 `FieldSample` 枚举和 `parse_and_resolve` 函数；`pipeline/pipeline.rs` 重命名为 `orchestrator.rs` 消除 clippy `module_inception` 警告。

---

## POC 关键差距 (POC Critical Gaps)

> **评估日期**: 2026-05-09
> **评估范围**: 当前 CPML v0.1.0 与 POC 里程碑目标之间的关键功能差距

### P0 — 阻塞 POC 演示

#### 1. [已修复] 几何体布尔组合 (Geometry Boolean Operations)

- **来源**: README, `adr-geometry.md`
- **问题描述**: ~~README 描述投影几何支持 "union/intersect/subtract" 布尔组合，但 Schema 和 Model 中均未实现。~~
- **修复记录**: 已于 2026-05-10 实现：(1) `ShapeDef`/`Shape` 新增 `Union`, `Intersection`, `Subtract` 变体；(2) `local_aabb()` 支持递归组合计算（Union 并集、Intersection 交集、Subtract 保守包围盒）；(3) `to_parry_shape()` Union 转为 CompoundShape；(4) `exact_intersects()` 实现 Intersection 全子形状求交、Subtract A-B 差集检测；(5) 重构 `orchestrator.rs` 中重复的几何解析代码，统一调用 `resolve::resolve_geometry()`。新增 10 个集成测试覆盖全部布尔运算场景。
- **状态**: 已修复

#### 2. [已修复] 多方案对比 (Scenario Comparison)

- **来源**: 产品定位 (AGENTS.md)
- **问题描述**: ~~Metric 系统已能量化单一方案的风险指数和成本影响，但 CLI 不支持多方案并排对比。~~
- **修复记录**: 已于 2026-05-10 实现：(1) 新增 `src/comparison.rs` 模块，`compare_results()` 计算 schedule/risk/cost 差异及唯一诊断项；(2) 新增 `src/output/compare.rs` 提供 text/JSON 格式化输出；(3) CLI 新增 `cpml compare <file_a> <file_b> [--format json]` 子命令。对比结果包含：工期天数差、风险指数差、成本影响差、各方案独有诊断项列表。
- **状态**: 已修复

#### 3. [已修复] 完整端到端演示场景 (End-to-End Demo Scenario)

- **来源**: POC 目标
- **问题描述**: ~~当前 6 个 sample 各自独立演示单一特性，缺少一个将所有能力串联的完整施工场景。~~
- **修复记录**: 已于 2026-05-10 创建 `samples/full_construction_demo.cpml`，包含 17 个活动构成的完整施工场景：两座塔吊调度（碰撞+遮挡）、5 阶段混凝土养护（标量递进）、行政审批（存在场）、供电资源池（容量场 + Structure 语法糖）、场地通行容量约束、材料流速监测（背压检测 Lte）、L 形建筑（布尔 Union）、带开口围挡（布尔 Subtract）、工序 FS/SS 依赖、分包商低置信度数据、子日精度混凝土浇筑。新增 9 个集成测试验证所有能力。
- **状态**: 已修复

### P1 — 削弱 POC 说服力

#### 4. [已修复] 显式工序依赖 (Explicit Predecessor/Successor)

- **来源**: `docs/examples.md` (#6 工序依赖)
- **问题描述**: ~~当前工序依赖通过 PresenceField 间接建模，无法表达 FS/SS/FF/SF 及 lag 时间约束。~~
- **修复记录**: 已于 2026-05-10 实现：(1) Schema 新增 `DependencyDef { activity_id, kind: FS|SS|FF|SF, lag_days }` 及 `depends_on: Vec<DependencyDef>` 到 `ActivityDef`；(2) Model 新增对应的 `Dependency`/`DependencyKind` 类型；(3) 解析阶段验证依赖引用存在性及自引用；(4) 新增 `src/pipeline/dependency_check.rs`，在关键帧级别检查 FS/SS/FF/SF 约束并生成 Error 诊断；(5) 支持正/负 lag 天数。新增 8 个集成测试覆盖所有依赖类型及边界条件。
- **状态**: 已修复

#### 5. [已修复] Structure 语法糖不支持 Confidence 配置

- **来源**: `adr-structure-expansion.md`
- **问题描述**: ~~Structure 展开时 `confidence` 硬编码为 `None`。~~
- **修复记录**: 已于 2026-05-10 (1) `StructureDef` 新增 `confidence: Option<f64>` 字段；(2) `expand.rs` 中透传至生成的 `Projection.confidence`。无需额外推理逻辑——已有 `eval_capacity`/`eval_scalar`/`eval_rate` 自动按 confidence 加权。新增 2 个集成测试。
- **状态**: 已修复

### P2 — 可观察但不阻塞 POC

#### 6. [已修复] 子日时间精度 (Sub-Day Temporal Resolution)

- **来源**: `adr-keyframes.md`
- **问题描述**: ~~当前时间精度为日期级别（`NaiveDate`），无法建模短于一天的活动。~~
- **修复记录**: 已于 2026-05-10 (1) `Timespan`/`Keyframe`/`Diagnostic.keyframe_date`/`MetricPoint.keyframe_date` 全面升级为 `NaiveDateTime`；(2) 解析器支持完整 ISO 8601 日期时间（`2026-03-12T08:00:00`），自动回退到纯日期格式（默认午夜）；(3) `PresenceRecord` 日期保持 `NaiveDate`（日历级别字段），比较时通过 `.date()` 转换；(4) 输出格式化智能显示——午夜显示日期，非午夜显示时间。新增 4 个集成测试覆盖子日精度、混合格式、半开工区间。
- **状态**: 已修复

#### 7. [已修复] RateField 背压/饥饿端到端验证

- **来源**: `docs/examples.md` (#7 流水线依赖)
- **问题描述**: ~~`eval_rate` 返回速率值但缺少与容量上限/下限比较的断言逻辑。~~
- **修复记录**: 已于 2026-05-10 (1) `AssertionDef`/`Assertion` 新增 `Lte(f64)` 和 `Range { min, max }` 变体，适用于 Capacity/Scalar/Rate 全部数值场类型；(2) `probe_check.rs` 提取 `sample_field_value()` 辅助函数消除重复求值代码，新增 Lte/Range 诊断生成；(3) `resolve.rs` 验证 Lte/Range 仅用于数值场类型；(4) `ratefield_demo.cpml` 扩展为 4 个场景——基础流速追踪、置信度加权、背压检测（Lte）、饥饿检测（Range）。新增 4 个集成测试覆盖 Lte/Range 通过/失败场景。
- **状态**: 已修复

---

## 代码级技术债 (续)

### 4. [已修复] Blame 归因包含未活跃的未来投影

- **发生位置**: `pipeline/probe_check.rs`
- **问题描述**: `build_occupancy_blame`、`build_value_blame`、`build_presence_blame` 三个归因构建函数接收 `&all_projections`（所有 activity 的全部投影，含未来/不活跃的），导致诊断报告中 blame 列表列出尚未开始的未来活动。虽然实际场求值（`eval_*`）仅使用 `active_projections`，数值正确，但 blame 输出造成误导——用户会看到 "north_road_closure 导致第 0 关键帧的 road_access 不足"，而 north_road_closure 在第 8 周才开始。
- **发现场景**: AIDC 柴发小楼 POC 模型 (`samples/aidc_diesel_generator_building.cpml`)，`north_road_closure` 和 `west_road_closure`（第 6-7 周）出现在第 0 关键帧的 road_access 诊断 blame 中。
- **修复记录**: 已于 2026-05-10 将三个 blame 函数的入参从 `&all_projections` 改为 `&active_projections`（经 active_projection_ids 过滤的当前关键帧活跃投影），修复后 blame 仅列当前关键帧实际在场值有贡献的投影。
- **状态**: 已修复

### 5. [已修复] 日期时间解析不支持分钟精度

- **发生位置**: `pipeline/resolve.rs` → `parse_datetime()`
- **问题描述**: `parse_datetime` 仅支持秒精度（`2026-03-12T08:00:00`）、带时区（`2026-03-12T08:00:00+08:00`）和纯日期（`2026-03-12`）三种格式。LLM 在生成半天粒度的施工计划时自然使用了分钟精度格式（`2026-06-01T08:00`），导致解析失败 `trailing input`。
- **发现场景**: AIDC 柴发小楼 POC 模型 — 这是 POC 中唯一的编译阻断错误，也是 LLM 从 schema 类型（`String`）无法推断的隐含格式约束。
- **修复记录**: 已于 2026-05-10 在 `parse_datetime` 中添加 `%Y-%m-%dT%H:%M` 格式支持，作为秒精度和纯日期间的第二回退尝试。
- **状态**: 已修复

### 6. [已修复] Capacity 场求值未按 field_name 过滤导致跨场污染

- **发生位置**: `pipeline/field_eval.rs` → `eval_capacity()`
- **问题描述**: `eval_capacity` 对所有活跃的 Capacity 投影求和时，未按 `field_name` 过滤。多个容量场并存时（如 `power_supply` 和 `water_supply`），一个场的探针会累加另一个场的投影贡献，导致探针实测值包含不相关场的数据。
- **发现场景**: `samples/demands_demo.cpml` — power_supply（+500）和 water_supply（+100）并存，water_supply 探针错误地将 power_supply 的 +500 计入采样值，导致所有水资源探针误判为通过。
- **修复记录**: 已于 2026-05-10 为 `eval_capacity` 新增 `field_name: &str` 参数，在投影过滤中增加 `p.field_name == field_name` 条件。同步更新 `probe_check.rs` 中 `sample_field_value` 的调用点。
- **状态**: 已修复

---

## POC 验证发现的设计改进

> **评估日期**: 2026-05-10
> **评估场景**: AIDC 备用柴发小楼 — 混凝土底座+钢结构混合建筑，8 柴发方舱+1 变电方舱，道路封闭，材料预囤，管廊接入
> **状态**: 已于 2026-05-10 创建 `adr-activity-boundary.md`，给出原子活动拆分准则框架与活动模板定位。

### D1. 原子活动拆分准则

- **问题描述**: AIDC POC 模型暴露了"何时拆分为 series、何时保持单个原子活动"缺少明确判断依据。道路封闭活动满足"空间/资源/风险恒定 + 时间短 + 内部细节对周边无差异化影响"四条件，应保持原子；混凝土养护每周强度不同，必须拆分。
- **决议**: 见 `docs/adr/adr-activity-boundary.md` 第一部分。四条件全满足 → 原子活动；任一违反 → 拆分。
- **优先级**: P1 — 影响模型一致性和 LLM 生成质量
- **状态**: 已决议，编译期暂不强制校验（由建模者遵循）

### D2. 活动模板 — 不可剥离风险包络

- **问题描述**: 钢结构焊接不仅是一个空间占用活动，它固有地携带火源投影（电弧球体 + 熔渣柱体）、可燃气体敏感探针（封闭空间 VOC）、登高风险（info 级，不可消除）、焊工需求、作业净空要求。如果计划工程师只写 `collision.hard`，遗漏这些并不是"建模不完整"——是物理规律不允许遗漏。需要机制确保这些不可剥离要素被强制携带。
- **决议**: 见 `docs/adr/adr-activity-boundary.md` 第二部分。活动模板 = 预声明的完整 probe + projection 包络，代表某类施工操作固有的、不可剥离的风险与需求组合。与语法糖不同——语法糖是简写，模板是知识缺省填充。
- **优先级**: P1 — 直接影响风险建模完整性
- **状态**: 概念定义完成，模板 DSL 语法和展开机制待设计

---

## 工具链与工程化技术债 (Toolchain & Engineering Maturity)

> **评估日期**: 2026-05-09  
> **修复日期**: 2026-05-09  
> **评估范围**: CPML v0.1.0 工具链全貌（CI/CD、质量保障、依赖安全、性能工程、发布管理、开发者体验）  

### P0 — 阻塞交付质量

#### 1. [已修复] CI/CD 流水线完全缺失

- **问题描述**: ~~不存在任何 CI 配置文件。每次 push/PR 依赖开发者手动运行检查，无法阻止回归进入主分支。~~
- **修复记录**: 已于 2026-05-09 创建 `.github/workflows/ci.yml`，覆盖三个 job：(1) `build` — cargo build + test + clippy + fmt-check；(2) `coverage` — cargo-tarpaulin + Codecov 上报；(3) `audit` — cargo-deny 依赖审计。使用 `dtolnay/rust-toolchain` + `Swatinem/rust-cache` 加速构建。
- **状态**: 已修复

#### 2. [已修复] 测试覆盖率度量缺失

- **问题描述**: ~~无 `cargo-tarpaulin`、`grcov`、`codecov` 等覆盖率工具配置。25 个测试的覆盖范围不可知。~~
- **修复记录**: 已于 2026-05-09 在 CI `coverage` job 中集成 `cargo-tarpaulin`，生成 XML 报告并上传 Codecov。`justfile` 中提供 `just coverage` 本地快捷命令。
- **状态**: 已修复

#### 3. [已修复] 依赖安全审计缺失

- **问题描述**: ~~无 `cargo-deny` 或 `cargo-audit` 配置，对 6 个直接依赖及传递依赖链的安全性无可见性。~~
- **修复记录**: 已于 2026-05-09 创建 `deny.toml`，配置：(1) `[advisories]` — 漏洞/unsound/yanked 设为 deny；(2) `[licenses]` — 允许 OSI/FSF 自由许可，copyleft 拒绝；(3) `[bans]` — 禁止多版本共存与 wildcard 依赖；(4) `[sources]` — 禁止 unknown-registry/unknown-git。CI `audit` job 使用 `EmbarkStudios/cargo-deny-action@v2` 自动运行。
- **状态**: 已修复

### P1 — 影响工程规范

#### 4. [已修复] Release Profile 未优化

- **问题描述**: ~~`Cargo.toml` 无 `[profile.release]`、`[profile.dev]` 配置段，所有 profile 使用 Rust 默认值。~~
- **修复记录**: 已于 2026-05-09 在 `Cargo.toml` 添加：(1) `[profile.release]` — `lto = "thin"`, `codegen-units = 1`, `panic = "abort"`；(2) `[profile.dev]` — `opt-level = 1`。
- **状态**: 已修复

#### 5. [已修复] 性能基准测试缺失

- **问题描述**: ~~无 `benches/` 目录、无 `criterion` 依赖、无 `#[bench]` 函数。碰撞检测（GJK）、场求值、遮挡剔除（raycasting）路径无回归检测。~~
- **修复记录**: 已于 2026-05-09 引入 `criterion` v0.5（dev-dependency + `[[bench]]` harness），创建 `benches/pipeline_benchmarks.rs` 覆盖 6 条基准路径：(1) parse + resolve 往返；(2-6) 5 个 sample 文件的完整流水线（resource_contention, collision_demo, occlusion_demo, scalar_progression, ratefield_demo）。
- **状态**: 已修复

#### 6. [已修复] 发布自动化缺失

- **问题描述**: ~~无 `cargo-release` 配置、无版本号管理脚本。版本号、git tag、changelog 无流水线串联。~~
- **修复记录**: 已于 2026-05-09 创建 `.release.toml`，配置 `cargo-release`：tag-name = `v{{version}}`，pre-release-commit-message，dev-version 自动 bump。使用 `cargo release patch|minor|major` 一键发布。
- **状态**: 已修复

### P2 — 影响开发者体验

#### 7. [已修复] Pre-commit Hook 安装未自动化

- **问题描述**: ~~hook 脚本存在于 `scripts/pre-commit`，需手动复制到 `.git/hooks/pre-commit`。~~
- **修复记录**: 已于 2026-05-09 在 `justfile` 中添加 `just setup` 目标，自动安装 `cargo-tarpaulin`、`cargo-deny`、`cargo-criterion`、`cargo-release` 工具链，并复制 pre-commit hook 到 `.git/hooks/`。
- **状态**: 已修复

#### 8. [已修复] EditorConfig / 编辑器配置缺失

- **问题描述**: ~~无 `.editorconfig`、`.gitignore` 仅忽略 `/target`。~~
- **修复记录**: 已于 2026-05-09 (1) 创建 `.editorconfig`：`charset = utf-8`, `indent_style = space`, `indent_size = 4`, `end_of_line = lf`, `trim_trailing_whitespace = true`，含 Markdown/YAML/Makefile 例外规则；(2) 更新 `.gitignore` 追加 IDE 临时文件（`.vscode/`, `.idea/`, `*.swp`, `*~`）、macOS `.DS_Store`、coverage 产物目录。
- **状态**: 已修复

#### 9. [已修复] 文档自动发布缺失

- **问题描述**: ~~`Cargo.toml` 无 `[package.metadata.docs.rs]` 配置，`cargo doc` 可本地生成但不会自动发布到 docs.rs。~~
- **修复记录**: 已于 2026-05-09 在 `Cargo.toml` 添加 `[package.metadata.docs.rs]` 段，`all-features = true`。待推送至 GitHub 后 crates.io 发布时将自动触发 docs.rs 构建。
- **状态**: 已修复

#### 10. [已修复] 任务运行器缺失

- **问题描述**: ~~无 Makefile、`justfile`、`cargo-make` 等任务运行器。常用命令需手敲。~~
- **修复记录**: 已于 2026-05-09 创建 `justfile`，提供 13 个快捷目标：`check`（clippy + test + fmt）、`lint`、`test`、`test-verbose`、`fmt-check`、`fmt`、`doc`、`coverage`、`audit`、`bench`、`setup`、`release`、`ci`（完整本地 CI 流水线）。
- **状态**: 已修复
