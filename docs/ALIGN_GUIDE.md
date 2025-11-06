# 对齐（Align）完全指南

本文档详细介绍 HBox 和 VBox 的对齐功能，让你无需使用 `<Spacer flex="1" />` 就能实现各种布局。

---

## 基本概念

### HBox 对齐
- **主轴**（水平方向）：使用 `align` 属性
- **交叉轴**（垂直方向）：使用 `valign` 属性

### VBox 对齐
- **主轴**（垂直方向）：使用 `align` 属性
- **交叉轴**（水平方向）：使用 `halign` 属性

---

## HBox 对齐

### `align` - 水平对齐（主轴）

#### `align="left"` - 左对齐（默认）
```xml
<HBox align="left">
  <Button text="按钮1" />
  <Button text="按钮2" />
</HBox>
```
```
[按钮1] [按钮2]                    空白
```

#### `align="center"` - 居中
```xml
<HBox align="center">
  <Button text="按钮1" />
  <Button text="按钮2" />
</HBox>
```
```
        空白        [按钮1] [按钮2]        空白
```

#### `align="right"` - 右对齐
```xml
<HBox align="right">
  <Button text="取消" />
  <Button text="确定" />
</HBox>
```
```
                    空白        [取消] [确定]
```

#### `align="space-between"` - 两端对齐
```xml
<HBox align="space-between">
  <Button text="上一步" />
  <Button text="下一步" />
</HBox>
```
```
[上一步]            空白            [下一步]
```

### `valign` - 垂直对齐（交叉轴）

#### `valign="top"` - 顶部对齐（默认）
```xml
<HBox valign="top">
  <Label text="小文本" />
  <Label text="大文本" font_size="24" />
</HBox>
```

#### `valign="center"` / `valign="middle"` - 垂直居中
```xml
<HBox valign="center">
  <Image icon="logo.png" width="32" height="32" />
  <Label text="应用名称" />
</HBox>
```

#### `valign="bottom"` - 底部对齐
```xml
<HBox valign="bottom">
  <Label text="文本" />
  <Button text="按钮" height="40" />
</HBox>
```

---

## VBox 对齐

### `align` - 垂直对齐（主轴）

#### `align="top"` - 顶部对齐（默认）
```xml
<VBox align="top">
  <Label text="标题" />
  <Label text="内容" />
</VBox>
```

#### `align="center"` / `align="middle"` - 垂直居中
```xml
<VBox align="center">
  <Label text="居中显示的内容" />
  <Button text="按钮" />
</VBox>
```

#### `align="bottom"` - 底部对齐
```xml
<VBox align="bottom">
  <Label text="这些元素" />
  <Label text="会显示在" />
  <Label text="底部" />
</VBox>
```

#### `align="space-between"` - 上下两端对齐
```xml
<VBox align="space-between">
  <Label text="顶部标题" />
  <Label text="底部按钮" />
</VBox>
```

### `halign` - 水平对齐（交叉轴）

#### `halign="left"` - 左对齐（默认）
```xml
<VBox halign="left">
  <Label text="左对齐文本" />
  <Button text="按钮" />
</VBox>
```

#### `halign="center"` - 水平居中
```xml
<VBox halign="center">
  <Label text="居中标题" />
  <Label text="居中副标题" />
</VBox>
```

#### `halign="right"` - 右对齐
```xml
<VBox halign="right">
  <Label text="右对齐" />
  <Button text="按钮" />
</VBox>
```

---

## 实际应用场景

### 场景 1: 右对齐按钮组
```xml
<!-- 旧方式（冗余） -->
<HBox>
  <Spacer flex="1" />
  <Button text="取消" />
  <Spacer width="10" />
  <Button text="确定" />
</HBox>

<!-- 新方式（简洁） -->
<HBox align="right" spacing="10">
  <Button text="取消" text_i18n="button.cancel" />
  <Button text="确定" text_i18n="button.ok" />
</HBox>
```

### 场景 2: 居中标题
```xml
<!-- 旧方式 -->
<HBox>
  <Spacer flex="1" />
  <Label text="欢迎" font_size="24" />
  <Spacer flex="1" />
</HBox>

<!-- 新方式 -->
<HBox align="center">
  <Label text="欢迎" text_i18n="welcome.title" font_size="24" />
</HBox>
```

### 场景 3: Logo + 文本（垂直居中）
```xml
<HBox valign="center" spacing="10">
  <Image icon="logo.png" width="48" height="48" />
  <Label text="应用名称" text_i18n="app.name" font_size="20" />
</HBox>
```

### 场景 4: 两端对齐导航
```xml
<HBox align="space-between">
  <Button text="上一步" text_i18n="button.back" />
  <Button text="下一步" text_i18n="button.next" />
</HBox>
```

### 场景 5: 垂直居中的欢迎页
```xml
<VBox align="center" halign="center">
  <Image icon="logo.png" width="128" height="128" />
  <Spacer height="20" />
  <Label text="欢迎使用 TapTap" text_i18n="welcome.title" font_size="24" />
  <Label text="点击下一步开始安装" text_i18n="welcome.subtitle" />
</VBox>
```

### 场景 6: 底部按钮栏
```xml
<VBox>
  <!-- 主要内容 -->
  <VBox flex="1">
    <Label text="安装内容..." />
  </VBox>
  
  <!-- 底部按钮 -->
  <HBox align="right" spacing="10" padding="10">
    <Button text="取消" text_i18n="button.cancel" />
    <Button text="安装" text_i18n="button.install" />
  </HBox>
</VBox>
```

### 场景 7: 表单布局（标签+输入框）
```xml
<HBox valign="center" spacing="10">
  <Label text="安装路径：" text_i18n="config.path" width="80" />
  <TextInput id="install_path" flex="1" />
  <Button text="浏览..." text_i18n="config.browse" width="80" />
</HBox>
```

---

## 对齐属性速查表

### HBox

| 属性 | 可选值 | 说明 | 默认值 |
|------|--------|------|--------|
| `align` | `left`, `center`, `right`, `space-between` | 水平对齐（主轴） | `left` |
| `valign` | `top`, `center`/`middle`, `bottom` | 垂直对齐（交叉轴） | `top` |

### VBox

| 属性 | 可选值 | 说明 | 默认值 |
|------|--------|------|--------|
| `align` | `top`, `center`/`middle`, `bottom`, `space-between` | 垂直对齐（主轴） | `top` |
| `halign` | `left`, `center`, `right` | 水平对齐（交叉轴） | `left` |

---

## 与 Flex 的对比

### ✅ 推荐使用 `align`（90% 的场景）

```xml
<!-- 简洁、清晰 -->
<HBox align="right">
  <Button text="确定" />
</HBox>
```

### ⚠️ 保留使用 `flex`（10% 的复杂场景）

```xml
<!-- 输入框占满剩余空间 -->
<HBox>
  <Label text="路径：" />
  <TextInput id="path" flex="1" />
  <Button text="浏览" />
</HBox>
```

---

## 最佳实践

### ✅ 推荐做法

1. **优先使用 `align`**：
   ```xml
   <HBox align="right">
     <Button text="确定" />
   </HBox>
   ```

2. **需要元素占据空间时用 `flex`**：
   ```xml
   <TextInput id="input" flex="1" />
   ```

3. **组合使用**：
   ```xml
   <HBox align="space-between" valign="center">
     <Label text="标题" />
     <Button text="按钮" />
   </HBox>
   ```

### ❌ 避免做法

1. **避免使用 `<Spacer flex="1" />`**：
   ```xml
   <!-- 不好 -->
   <HBox>
     <Spacer flex="1" />
     <Button text="确定" />
   </HBox>
   
   <!-- 好 -->
   <HBox align="right">
     <Button text="确定" />
   </HBox>
   ```

2. **避免嵌套多层容器来实现对齐**：
   ```xml
   <!-- 不好 -->
   <VBox>
     <Spacer flex="1" />
     <HBox>
       <Spacer flex="1" />
       <Label text="居中" />
       <Spacer flex="1" />
     </HBox>
     <Spacer flex="1" />
   </VBox>
   
   <!-- 好 -->
   <VBox align="center" halign="center">
     <Label text="居中" />
   </VBox>
   ```

---

## 完整示例

### 完整的安装向导页面

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="InstallConfig" version="1.0.0">
  <Page>
    <!-- 整体垂直布局 -->
    <VBox padding="30" spacing="20">
      
      <!-- 标题（居中） -->
      <HBox align="center">
        <Label text="安装配置" 
               text_i18n="config.title" 
               font_size="24" />
      </HBox>
      
      <!-- 安装路径（标签+输入框+按钮） -->
      <VBox spacing="5">
        <Label text="安装路径" text_i18n="config.path_label" />
        <HBox valign="center" spacing="10">
          <TextInput id="install_path" flex="1" />
          <Button text="浏览..." text_i18n="config.browse" width="80" />
        </HBox>
      </VBox>
      
      <!-- 选项列表（左对齐） -->
      <VBox spacing="10" halign="left">
        <Checkbox id="desktop" 
                  text="创建桌面快捷方式" 
                  text_i18n="config.desktop" />
        <Checkbox id="startmenu" 
                  text="添加到开始菜单" 
                  text_i18n="config.startmenu" />
      </VBox>
      
      <!-- 占据剩余空间 -->
      <Spacer flex="1" />
      
      <!-- 底部按钮（右对齐） -->
      <HBox align="right" spacing="10">
        <Button text="上一步" 
                text_i18n="button.back" 
                min_width="100" />
        <Button text="安装" 
                text_i18n="button.install" 
                min_width="100" />
      </HBox>
      
    </VBox>
  </Page>
</Layout>
```

---

## 常见问题

**Q: `align` 和 `flex` 可以同时使用吗？**
A: 如果有 `flex` 子元素，`align` 会被忽略。建议二选一。

**Q: `space-between` 只有一个子元素会怎样？**
A: 表现和 `left`/`top` 一样。

**Q: `valign` 和 `halign` 可以缩写吗？**
A: 不可以，必须完整拼写。

**Q: 对齐不生效？**
A: 检查：
1. 是否有 `flex` 子元素（会切换到 flex 模式）
2. 容器是否有足够的空间
3. 属性名是否拼写正确


