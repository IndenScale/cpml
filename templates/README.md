# CPML Activity Templates

预定义的、携带完整 probe + projection + demands 包络的活动模板。每个模板代表某类施工操作固有的、不可剥离的风险与需求组合。

## 使用方式

模板使用 `{{param}}` 占位符，实例化时需由上游工具（LLM 或 CLI）替换为具体值：

1. 复制模板内容
2. 将 `{{id}}` 替换为唯一活动 ID
3. 将 `{{timespan_start}}` / `{{timespan_end}}` 替换为 ISO 8601 日期时间
4. 将 `{{work_point}}`、`{{work_envelope}}` 等几何引用替换为 `.cpml` 文件中定义的 geometry ID
5. 确保父 `.cpml` 文件声明了模板所需的所有 field 类型（见每个模板文件头部的 Required fields 列表）

## 模板列表

| 模板 | 风险域 | 不可剥离要素 |
|------|--------|-------------|
| `fire/welding_arc` | fire, fall | 电弧火源、熔渣火源、可燃气体敏感、登高交底、焊工需求、作业净空 |
| `fall/work_at_height` | fall | 坠落风险区、登高设备需求、作业净空、登高交底 |
| `mechanical/lift_crane` | mechanical, fall | 摆臂 soft zone、荷载坠落风险、信号工需求、吊运通道净空、风速敏感 |
| `fire/fuel_handling` | fire | fire_load 注入、火险半径 soft zone、围堰需求、火源敏感、消防通道 |
| `energy/commissioning_hot` | energy, fire | 最高 fire_load、意外启动风险、LOTO 验证、人员清场、消防响应需求 |

## 为什么不是语法糖

`collision` 和 `structure` 是语法糖 —— 对已有原语的简写。

活动模板是**知识缺省填充** —— 施工计划工程师可能不知道"焊接必然携带火源+可燃气体敏感+登高风险"这条因果链，但物理规律不会遗漏。模板确保这个因果链在 CPML 编译期暴露，而非在施工现场暴露。
