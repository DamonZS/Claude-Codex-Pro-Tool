# Multica 独立标签资源

## 目标
接入上游 Label 的独立本地集合，支持 issue、agent、skill 三种资源类型，名称、描述和标准十六进制颜色，并使用通用 revision CAS。

## 约束
标签不得并入 Issue JSON；仅提供本地控制面 CRUD，不伪造远端标签 API 或通知同步。
