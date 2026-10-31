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
   * 请求地址。占位符由后端在发请求前替换：
   * - `{limit}`：本次要几张图（当前页大小）
   * - `{page}`：页码，从 1 开始
   * - `{pageIndex}`：`page - 1`，给"页码从 0 开始"的接口用（Gelbooru 的 `pid`）
   * - `{offset}`：`(page - 1) * limit`，给偏移量分页的接口用（360 壁纸的 `start`）
   * - `{query}`：搜索关键词（只有在 `search.url` 里才有意义）
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
  /** 原图宽度的字段路径；省略 = 该图源不提供尺寸，分辨率筛选对它无效。 */
  width?: string;
  /** 原图高度的字段路径；规则同 `width`。 */
  height?: string;
  /** 合在一起的尺寸字段，形如 `"2560x1440"`；配了 width/height 时以后者为准。 */
  resolution?: string;
}

/**
 * 图源的附加过滤。
 *
 * 注意这**不是** `sfwOnly: true` 那种布尔开关。布尔字段看起来像保护，
 * 但只要没有代码消费它，它就是一句口号——比没有更糟。
 *
 * 真正起作用的是请求参数，所以这里存的是"要注入的参数"，由后端拼到地址上。
 */
export interface SourceFilter {
  /**
   * 由后端注入到请求地址末尾的安全参数，例如 `purity=100`、`r18=0`。
   *
   * 只写裸参数，不要带 `?` 或 `&`。需要内容限制的图源必须写；
   * 本身只提供安全内容的图源留空。
   */
  sfwQuery?: string;
  /**
   * 把图片地址从 `http://` 升级成 `https://`。
   * 界面跑在安全上下文里，WebView 不会加载 http 图片（表现为"列表有但图全裂"）。
   */
  httpsUpgrade?: boolean;
  /**
   * 把 `{offset}` 换成随机起点（`limit` 的倍数）。
   * 给"顺序固定、支持偏移量分页"的图源用，让"翻页"变成"换一批"。
   * 值 = 想覆盖的总量（接口 total 10000、每页 20 → 填 8000）。
   */
  randomOffsetMax?: number;
  /**
   * 把 `{pageIndex}` 换成随机页码。给"页码从 0 开始"的图源用。
   *
   * **确定性列表必须配它**：safebooru 的 `pid=0` 永远是最新 20 张，
   * 不配的话用户每次打开看到的都是同一批图——界面上这和"缓存坏了"没法区分。
   *
   * 值 = 想覆盖的页数，但别贪大：safebooru 实测 `pid=10000` 正常、
   * `pid=15000` 起返回 XML 的 `API limited due to abuse`，所以填 8000。
   */
  randomPageIndexMax?: number;
}

/**
 * 搜索能力。
 *
 * **不配这个块 = 该图源不支持搜索**，界面上搜索框要对它禁用。
 * 这比"配了但静默无效"好：用户能看见边界在哪。
 */
export interface SearchSpec {
  /**
   * 搜索用的完整地址模板，**必须包含 `{query}`**。
   *
   * `{page}` / `{limit}` 同样可用。关键词由后端编码后填入。
   * 之所以不复用 `request.url`：有些站点的搜索是另一个端点
   * （nekos.best 浏览走 `/api/v2/neko`，搜索走 `/api/v2/search` 且必须带 `type`）。
   */
  url: string;
  /**
   * 关键词要不要先翻成英文（走 `search/terms.json` 词表）。
   *
   * - `true`：wallhaven / nekos.best 只认英文
   * - `false`：lolicon 走 pixiv 标签，中文直接可用
   */
  english?: boolean;
  /**
   * 关键词里的空白要不要换成下划线。
   *
   * **booru 类站点必须开**：标签是 `hatsune_miku` 形态、词间是 AND，
   * 发 `hatsune miku` 会被当成两个标签同时命中（实测 0 条），
   * 而词表里有 43/149 条译名带空格（初音 → `hatsune miku`）。
   * 不开这个，中文搜索逢多词必 0 条。
   */
  underscoreTags?: boolean;
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
  /** 不配 = 不支持搜索。 */
  search?: SearchSpec;
}

/** `sources/builtin/*.json` 的顶层结构。 */
export interface SourcesFile {
  schemaVersion: 1;
  sources: WallpaperSource[];
}
