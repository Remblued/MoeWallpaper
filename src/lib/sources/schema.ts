/**
 * 图源（source）配置的类型契约 —— 对应 `sources/builtin/*.json`。
 *
 * 设计目标：加一个图源只改 JSON，不改代码。因此这里的类型必须覆盖
 * "请求怎么发、响应怎么读、要不要过滤"三件事，其它一律不做。
 *
 * 修改本文件 = 修改图源格式，属于破坏性变更：
 * 需要同步改 `schemaVersion`、`sources/builtin/` 下的文件，以及 README 的示例。
 */

/** HTTP 请求描述。 */
export interface HttpRequestSpec {
  /**
   * 请求地址。支持两个占位符，由代码在发请求前替换：
   * - `{limit}`：本次要几张图（当前页大小）
   * - `{page}`：页码，从 1 开始
   */
  url: string;
  method?: "GET" | "POST";
  headers?: Record<string, string>;
}

/**
 * 响应映射：告诉代码"列表在哪、每张图的字段叫什么"。
 *
 * 路径用点号表示层级，例如 `data` 或 `data.posts`。
 * 字段值同样是点号路径，例如 `file_url` 或 `images.original.url`。
 * 不写这些映射，等于要求我们为每个 API 写一份适配代码 —— 那就退回插件模式了。
 */
export interface ResponseMapping {
  /** 图片数组在响应 JSON 里的位置；省略表示响应本身就是数组。 */
  itemsPath?: string;
  /** 原图 URL（必填）。 */
  imageUrl: string;
  /** 缩略图 URL；省略则用原图。 */
  thumbnailUrl?: string;
  /** 作品/详情页 URL，用于标注来源与作者。 */
  pageUrl?: string;
  /** 标签字段，用于展示和二次过滤。 */
  tags?: string;
}

/** 过滤规则。默认 SFW：宁可少给几张图，也不给不该给的内容。 */
export interface SourceFilter {
  /** 是否只接受被标记为安全的内容，默认 true。 */
  sfwOnly?: boolean;
}

/** 单个图源。 */
export interface WallpaperSource {
  /** 稳定标识，用于缓存目录名和用户配置引用；发布后不要改。 */
  id: string;
  /** 界面上显示的名字。 */
  name: string;
  /** 是否启用，默认 false —— 新图源由用户主动打开。 */
  enabled?: boolean;
  /** 站点主页，用于在界面上标注来源。 */
  homepage?: string;
  request: HttpRequestSpec;
  response: ResponseMapping;
  filter?: SourceFilter;
}

/** `sources/builtin/*.json` 的顶层结构。 */
export interface SourcesFile {
  schemaVersion: 1;
  sources: WallpaperSource[];
}
