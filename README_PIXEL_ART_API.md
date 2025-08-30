# 像素艺术创作系统 API 文档

## 概述

像素艺术创作系统提供了完整的像素艺术项目管理功能，包括创建、版本控制、存储和导出功能。

## 数据类型

### ProjectId
```candid
type ProjectId = text;
```
项目唯一标识符，格式：`proj_{timestamp}_{random}`

### VersionId  
```candid
type VersionId = text;
```
版本唯一标识符，格式：`ver_{timestamp}_{random}`

### PixelArtSource
```candid
type PixelArtSource = record {
  width: nat32;
  height: nat32;
  palette: vec text;           // HEX颜色值，如 "#FF0000"
  pixels: vec (vec nat16);     // 调色板索引矩阵
  frames: opt vec Frame;       // 可选动画帧
  metadata: opt SourceMeta;    // 可选元数据
};
```

### Frame
```candid
type Frame = record {
  duration_ms: nat32;          // 帧持续时间（毫秒）
  pixels: vec (vec nat16);     // 该帧的像素矩阵
};
```

### Project
```candid
type Project = record {
  project_id: ProjectId;
  owner: principal;
  created_at: nat64;           // 创建时间戳（秒）
  updated_at: nat64;           // 最后更新时间戳（秒）
  current_version: Version;
  history: vec Version;        // 版本历史
};
```

## API 接口

### 创建项目
```candid
create_pixel_project: (PixelArtSource, opt text) -> (variant { Ok: ProjectId; Err: text });
```

**参数：**
- `source`: 像素艺术源数据
- `message`: 可选的版本消息

**返回：**
- 成功：项目ID
- 失败：错误信息

**示例调用：**
```javascript
const source = {
  width: 16,
  height: 16,
  palette: ["#000000", "#FFFFFF", "#FF0000"],
  pixels: [
    // 16x16的像素矩阵
  ],
  frames: null,
  metadata: {
    title: "我的像素艺术",
    description: "第一个作品",
    tags: ["pixel", "art"]
  }
};

const result = await actor.create_pixel_project(source, "初始版本");
```

### 保存新版本
```candid
save_pixel_version: (ProjectId, PixelArtSource, opt text, opt text) -> (variant { Ok: VersionId; Err: text });
```

**参数：**
- `project_id`: 项目ID
- `source`: 新版本的像素艺术数据
- `message`: 可选的版本消息
- `if_match_version`: 可选的乐观锁版本ID

**示例调用：**
```javascript
const newSource = {
  // 修改后的像素艺术数据
};

const result = await actor.save_pixel_version(
  projectId, 
  newSource, 
  "添加了新颜色", 
  currentVersionId  // 乐观锁
);
```

### 获取项目
```candid
get_pixel_project: (ProjectId) -> (opt Project) query;
```

### 获取特定版本
```candid
get_pixel_version: (ProjectId, VersionId) -> (opt Version) query;
```

### 导出设备格式
```candid
export_pixel_for_device: (ProjectId, opt VersionId) -> (variant { Ok: text; Err: text }) query;
```

**返回紧凑JSON格式：**
```json
{
  "type": "pixel_art@1",
  "width": 16,
  "height": 16,
  "palette": ["#000000", "#FFFFFF"],
  "pixels": [[0,1,0], [1,0,1]],  // 静态图片
  "frames": [                    // 或动画帧
    {
      "durationMs": 500,
      "pixels": [[0,1], [1,0]]
    }
  ]
}
```

### 列出用户项目
```candid
list_pixel_projects_by_owner: (principal, nat64, nat64) -> (vec Project) query;
```

**参数：**
- `owner`: 所有者Principal
- `offset`: 偏移量
- `limit`: 限制数量

### 删除项目
```candid
delete_pixel_project: (ProjectId) -> (variant { Ok: text; Err: text });
```

## 前端集成指南

### 1. 身份验证
确保用户已通过IC身份验证：

```javascript
import { AuthClient } from "@dfinity/auth-client";

const authClient = await AuthClient.create();
const isAuthenticated = await authClient.isAuthenticated();

if (!isAuthenticated) {
  await authClient.login({
    identityProvider: "https://identity.ic0.app",
    onSuccess: () => {
      // 认证成功，可以调用API
      initPixelArtApp();
    }
  });
}
```

### 2. Actor创建
```javascript
import { createActor } from "./declarations/aio-base-backend";

const actor = createActor(canisterId, {
  agentOptions: {
    identity: authClient.getIdentity(),
    host: process.env.DFX_NETWORK === "local" 
      ? "http://localhost:4943" 
      : "https://ic0.app"
  }
});
```

### 3. 错误处理
```javascript
try {
  const result = await actor.create_pixel_project(source, message);
  
  if ('Ok' in result) {
    const projectId = result.Ok;
    console.log("项目创建成功:", projectId);
  } else {
    console.error("创建失败:", result.Err);
  }
} catch (error) {
  console.error("网络错误:", error);
}
```

### 4. 数据验证
前端应实现基本验证：

```javascript
function validatePixelArtSource(source) {
  // 检查维度
  if (source.width <= 0 || source.height <= 0) {
    throw new Error("无效的画布尺寸");
  }
  
  // 检查像素矩阵
  if (source.pixels.length !== source.height) {
    throw new Error("像素矩阵高度不匹配");
  }
  
  source.pixels.forEach(row => {
    if (row.length !== source.width) {
      throw new Error("像素矩阵宽度不匹配");
    }
    
    row.forEach(colorIndex => {
      if (colorIndex >= source.palette.length) {
        throw new Error("调色板索引超出范围");
      }
    });
  });
}
```

## 限制和约束

- 最大载荷大小：1MB
- 画布最大尺寸：建议不超过512x512
- 调色板最大颜色数：建议不超过256种
- 动画帧数：建议不超过60帧

## 最佳实践

1. **乐观锁使用**：在保存版本时使用`if_match_version`参数避免冲突
2. **分页查询**：列出项目时使用适当的分页参数
3. **错误处理**：始终检查API返回的Result类型
4. **本地缓存**：对频繁访问的项目数据进行本地缓存
5. **压缩优化**：大型像素艺术可考虑使用颜色压缩技术
