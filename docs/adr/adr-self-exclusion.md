# ADR: 自排除规则

## 背景

当探针评估场状态时，需要考虑是否排除探针所属活动自身的投影贡献。不同类型的场有不同的答案。

## 决策

**占用场探针排除自身：** 物理直觉——"碰撞"意味着"其他物体进入我的空间"。塔吊不应检测到与自身的碰撞。实现方式：在 `eval_occupancy` 中过滤掉 `parent_activity_id == self_activity_id` 的投影。

**容量场/标量场/存在场探针包含自身：** 共享资源的直觉——活动应能看到自身对资源总量的消耗。例如，一个消耗 150kW 的施工活动，其探针检查"是否有足够的电力"时，应该看到包括自身消耗在内的总负载。

```rust
// OccupancyField: self-exclusion
active_projections
    .filter(|p| p.parent_activity_id != self_activity_id)

// CapacityField / ScalarField / PresenceField: no self-exclusion
active_projections
    // (no activity filter)
```

**为什么不对称？**

- 占用场建模的是空间互斥关系——自身不能"碰撞"自身
- 容量场建模的是共享资源池——自身消耗是总账的一部分
- 标量场和存在场继承容量场的逻辑——它们建模的是共享状态

## 后果

- 在占用场中，一个活动声明了 hard 碰撞投影但未声明 hard 探针时，其他活动的探针仍能检测到该活动的占用
- 如果未来引入"自碰撞"场景（如活动自身的不同部分相互干扰），需要在投影级别添加更细粒度的排除控制
- 归因（Blame）在 OccupancyField 中同样排除自身投影
