# ADR: v1.1 几何表示

## 背景

v1.0 仅支持 `aabb: {min, max}` 和 `sphere: {center, radius}`，其中球体被近似为 AABB。施工场景中需要支持更多几何体（圆柱、圆锥、半球）以及任意位姿（位置 + 朝向），以精确建模柱、塔吊臂、开挖坑等实际结构。

## 决策

### 形状 + 位姿分离

所有几何体定义为"局部形状 + 世界位姿"的组合：

```yaml
geometries:
  - id: "column"
    pose:
      position: [5, 5, 2]
      rotation: [0, 0, 0]    # Euler 角（度），ZYX 顺规
    cylinder:
      radius: 0.5
      half_height: 2.0
```

**位姿（Pose）：**

- `position: [x, y, z]` — 世界空间平移
- `rotation: [yaw, pitch, roll]` — Euler 角（度），ZYX 顺规（先绕 Z 转 yaw，再绕 Y 转 pitch，再绕 X 转 roll），默认 `[0, 0, 0]`
- 位姿可选：未提供时默认为原点、零旋转

**五种形状（均在局部坐标系中定义，原点为中心）：**

| 形状 | YAML 键 | 参数 | 局部空间含义 |
|---|---|---|---|
| 长方体 | `cuboid` | `half_extents: [dx, dy, dz]` | 原点为中心，各轴延伸 ±half_extents |
| 圆柱 | `cylinder` | `radius`, `half_height` | Z 轴对齐，范围 [-half_height, +half_height] |
| 球 | `sphere` | `radius` | 原点为中心 |
| 半球 | `hemisphere` | `radius` | 平面在 Z=0，半球面沿 +Z |
| 圆锥 | `cone` | `radius`, `half_height` | 底面在 Z=-half_height，锥顶在 Z=+half_height |

### 向后兼容

保留 `aabb: {min, max}` 语法，内部自动转换为：

```text
half_extents = (max - min) / 2
pose.position = (min + max) / 2
pose.rotation = [0, 0, 0]
```

如果同时指定了 `pose` 和 `aabb`，pose 优先用于位置（覆盖自动计算的 center）。

### 世界空间 AABB 计算

v1 仍使用 AABB 做重叠检测。通过以下方法将任意形状变换到世界空间 AABB：

1. 取形状的局部空间 AABB（8 个角点）
2. 使用 ZYX Euler 旋转矩阵（`R = Rz(yaw) × Ry(pitch) × Rx(roll)`）对 8 个角点做旋转变换
3. 取旋转后角点的 min/max，加上 `pose.position` 的平移

这是**保守估计**——旋转后的形状可能不完全填满其世界 AABB，因此重叠检测可能产生假阳性，但绝不会漏检（假阴性）。

**精确碰撞检测（GJK/SAT 等）推迟到后续版本。**

### v1 限制

- 不包含锥体、布尔组合、遮挡剔除
- AABB 重叠检测为 O(n×m)，适用于 v1 场景规模
- 旋转使用 Euler 角（非四元数），存在万向节锁风险——施工场景中极端旋转罕见，实际风险低

## 后果

- 用户可以精确定义柱体（圆柱）、塔吊臂覆盖范围（圆锥）、开挖坑（半球）等实际施工几何
- 位姿支持允许非轴对齐的长方体（如斜向布置的支撑结构）
- 世界 AABB 保守估计意味着两个相距很近但未真正重叠的旋转长方体可能被误报为碰撞
- 新增形状类型只需在 `ShapeDef` 和 `Shape` 中各加一个变体，并实现 `local_aabb()` 方法
