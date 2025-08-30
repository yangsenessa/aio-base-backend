/**
 * 像素艺术创作系统前端集成示例
 * 这个文件展示了如何在React/Vue/Angular等前端框架中集成像素艺术API
 */

// 1. 导入必要的依赖
import { AuthClient } from "@dfinity/auth-client";
import { createActor } from "./declarations/aio-base-backend";

class PixelArtService {
  constructor() {
    this.actor = null;
    this.authClient = null;
    this.canisterId = process.env.REACT_APP_CANISTER_ID; // 从环境变量获取
  }

  /**
   * 初始化服务
   */
  async initialize() {
    try {
      // 创建认证客户端
      this.authClient = await AuthClient.create();
      
      // 检查是否已认证
      const isAuthenticated = await this.authClient.isAuthenticated();
      
      if (isAuthenticated) {
        await this.setupActor();
        return { success: true, authenticated: true };
      } else {
        return { success: true, authenticated: false };
      }
    } catch (error) {
      console.error("初始化失败:", error);
      return { success: false, error: error.message };
    }
  }

  /**
   * 用户登录
   */
  async login() {
    return new Promise((resolve, reject) => {
      this.authClient.login({
        identityProvider: process.env.NODE_ENV === "development" 
          ? `http://localhost:4943/?canisterId=${process.env.REACT_APP_INTERNET_IDENTITY_CANISTER_ID}`
          : "https://identity.ic0.app",
        onSuccess: async () => {
          await this.setupActor();
          resolve({ success: true });
        },
        onError: (error) => {
          reject({ success: false, error });
        }
      });
    });
  }

  /**
   * 用户登出
   */
  async logout() {
    await this.authClient.logout();
    this.actor = null;
  }

  /**
   * 设置Actor
   */
  async setupActor() {
    const identity = this.authClient.getIdentity();
    
    this.actor = createActor(this.canisterId, {
      agentOptions: {
        identity,
        host: process.env.NODE_ENV === "development" 
          ? "http://localhost:4943" 
          : "https://ic0.app"
      }
    });
  }

  /**
   * 验证像素艺术数据
   */
  validatePixelArtSource(source) {
    if (!source) {
      throw new Error("像素艺术数据不能为空");
    }

    if (source.width <= 0 || source.height <= 0) {
      throw new Error("画布尺寸必须大于0");
    }

    if (source.width > 512 || source.height > 512) {
      throw new Error("画布尺寸过大，最大支持512x512");
    }

    if (!source.palette || source.palette.length === 0) {
      throw new Error("调色板不能为空");
    }

    if (source.palette.length > 256) {
      throw new Error("调色板颜色数量过多，最大支持256种颜色");
    }

    // 验证颜色格式
    const hexColorRegex = /^#[0-9A-Fa-f]{6}$/;
    source.palette.forEach((color, index) => {
      if (!hexColorRegex.test(color)) {
        throw new Error(`调色板第${index + 1}个颜色格式无效: ${color}`);
      }
    });

    // 验证像素矩阵
    if (!source.pixels || source.pixels.length !== source.height) {
      throw new Error("像素矩阵高度与画布高度不匹配");
    }

    source.pixels.forEach((row, rowIndex) => {
      if (row.length !== source.width) {
        throw new Error(`第${rowIndex + 1}行像素数量与画布宽度不匹配`);
      }

      row.forEach((colorIndex, colIndex) => {
        if (colorIndex < 0 || colorIndex >= source.palette.length) {
          throw new Error(`像素[${rowIndex}, ${colIndex}]的颜色索引${colorIndex}超出调色板范围`);
        }
      });
    });

    // 验证动画帧（如果存在）
    if (source.frames) {
      if (source.frames.length > 60) {
        throw new Error("动画帧数过多，最大支持60帧");
      }

      source.frames.forEach((frame, frameIndex) => {
        if (!frame.duration_ms || frame.duration_ms <= 0) {
          throw new Error(`第${frameIndex + 1}帧的持续时间无效`);
        }

        if (!frame.pixels || frame.pixels.length !== source.height) {
          throw new Error(`第${frameIndex + 1}帧的像素矩阵高度不匹配`);
        }

        frame.pixels.forEach((row, rowIndex) => {
          if (row.length !== source.width) {
            throw new Error(`第${frameIndex + 1}帧第${rowIndex + 1}行像素数量不匹配`);
          }
        });
      });
    }
  }

  /**
   * 创建像素艺术项目
   */
  async createProject(pixelArtData, message = "初始版本") {
    try {
      this.validatePixelArtSource(pixelArtData);
      
      const result = await this.actor.create_pixel_project(pixelArtData, [message]);
      
      if ('Ok' in result) {
        return {
          success: true,
          projectId: result.Ok,
          message: "项目创建成功"
        };
      } else {
        return {
          success: false,
          error: result.Err
        };
      }
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * 保存新版本
   */
  async saveVersion(projectId, pixelArtData, message = "", currentVersionId = null) {
    try {
      this.validatePixelArtSource(pixelArtData);
      
      const messageParam = message ? [message] : [];
      const versionParam = currentVersionId ? [currentVersionId] : [];
      
      const result = await this.actor.save_pixel_version(
        projectId,
        pixelArtData,
        messageParam,
        versionParam
      );
      
      if ('Ok' in result) {
        return {
          success: true,
          versionId: result.Ok,
          message: "版本保存成功"
        };
      } else {
        return {
          success: false,
          error: result.Err
        };
      }
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * 获取项目详情
   */
  async getProject(projectId) {
    try {
      const result = await this.actor.get_pixel_project(projectId);
      
      if (result && result.length > 0) {
        return {
          success: true,
          project: result[0]
        };
      } else {
        return {
          success: false,
          error: "项目不存在"
        };
      }
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * 获取用户的所有项目
   */
  async getUserProjects(offset = 0, limit = 20) {
    try {
      const identity = this.authClient.getIdentity();
      const principal = identity.getPrincipal();
      
      const projects = await this.actor.list_pixel_projects_by_owner(
        principal,
        BigInt(offset),
        BigInt(limit)
      );
      
      return {
        success: true,
        projects: projects
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * 导出设备格式
   */
  async exportForDevice(projectId, versionId = null) {
    try {
      const versionParam = versionId ? [versionId] : [];
      const result = await this.actor.export_pixel_for_device(projectId, versionParam);
      
      if ('Ok' in result) {
        return {
          success: true,
          exportData: JSON.parse(result.Ok)
        };
      } else {
        return {
          success: false,
          error: result.Err
        };
      }
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * 删除项目
   */
  async deleteProject(projectId) {
    try {
      const result = await this.actor.delete_pixel_project(projectId);
      
      if ('Ok' in result) {
        return {
          success: true,
          message: "项目删除成功"
        };
      } else {
        return {
          success: false,
          error: result.Err
        };
      }
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * 获取项目总数
   */
  async getTotalProjectCount() {
    try {
      const count = await this.actor.get_total_pixel_project_count();
      return {
        success: true,
        count: Number(count)
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }
}

// React Hook 示例
export function usePixelArtService() {
  const [service] = useState(() => new PixelArtService());
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    async function init() {
      const result = await service.initialize();
      if (result.success) {
        setIsAuthenticated(result.authenticated);
      }
      setIsLoading(false);
    }
    init();
  }, [service]);

  const login = async () => {
    setIsLoading(true);
    try {
      await service.login();
      setIsAuthenticated(true);
    } catch (error) {
      console.error("登录失败:", error);
    }
    setIsLoading(false);
  };

  const logout = async () => {
    await service.logout();
    setIsAuthenticated(false);
  };

  return {
    service,
    isAuthenticated,
    isLoading,
    login,
    logout
  };
}

// Vue Composition API 示例
export function usePixelArtServiceVue() {
  const service = new PixelArtService();
  const isAuthenticated = ref(false);
  const isLoading = ref(true);

  onMounted(async () => {
    const result = await service.initialize();
    if (result.success) {
      isAuthenticated.value = result.authenticated;
    }
    isLoading.value = false;
  });

  const login = async () => {
    isLoading.value = true;
    try {
      await service.login();
      isAuthenticated.value = true;
    } catch (error) {
      console.error("登录失败:", error);
    }
    isLoading.value = false;
  };

  const logout = async () => {
    await service.logout();
    isAuthenticated.value = false;
  };

  return {
    service,
    isAuthenticated: readonly(isAuthenticated),
    isLoading: readonly(isLoading),
    login,
    logout
  };
}

export default PixelArtService;
