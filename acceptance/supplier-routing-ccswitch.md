# 供应商与路由整改验收标准

对应规格：`spec/supplier-routing-ccswitch.md`

## 通过标准
- [ ] `settings.json` 内容损坏时，加载流程会生成 `settings.json.corrupt-*.bak` 副本并记录错误。损坏前的原始内容必须可恢复，有单测覆盖。
- [ ] `provider_endpoint::resolve` 对以下情况返回正确的上游地址：Chat 协议优先取 `upstreamBaseUrl`；地址在 `config_contents` 中时也能取到；不会返回本地转换代理地址。有单测覆盖。
- [ ] 首次启动迁移：迁移前存在备份；数据库中的供应商数量、各应用当前供应商与 JSON 一致；重复执行迁移不会重复导入；迁移失败时回退读 JSON。有单测覆盖。
- [ ] 切换供应商：
  - Claude 的 `settings.json` 中非关键字段在切换后保持不变。
  - 上一家独有的字段被删除。
  - 第一次写入前生成备份。
  - 写入失败时 live 文件与设置都还原。
  - 以上均有单测覆盖。
- [ ] 熔断器三态转换、故障转移队列顺序、错误分类（400/405/406/413/414/415/422/501 不切换）均有单测。
- [ ] 连通检测：对本地 mock 服务验证超时 8s、重试 1 次、"较慢"阈值 6s。
- [ ] 阻塞命令已全部改为 `spawn_blocking`，可用代码搜索证明。
- [ ] 前端：
  - `SupplierScreen` 已拆分。
  - 顶栏与供应商页读取同一数据源。
  - JSX 中没有原样显示的 `\u` 转义。
  - 死代码已删除。
- [ ] `npm run check`、`vite build`、`cargo fmt --check`、core 与 manager 的相关 `cargo test`、`windows_subsystem` 全部通过。

## 必需证据
- 上述各命令的输出。
- 供应商页与设置-路由页的深色、浅色截图。

## 非目标
- 云同步。
- OAuth 认证中心。
