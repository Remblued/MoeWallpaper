/**
 * 图源（source）配置的类型契约 —— 对应 `sources/builtin/*.json`
 * 和用户自己的配置文件。
 *
 * 设计目标：加一个图源只改 JSON，不改代码。因此这里的类型必须覆盖
 * "请求怎么发、响应怎么读、要不要过滤"三件事，其它一律不做。
 *
 * 修改本文件 = 修改图源格式，属于破坏性变更：
 * 需要同步改 `schemaVersion`、`sources/builtin/` 下的文件、
 * `src-tauri/src/sources/model.rs`、`validate.rs` 的字段表，以及 README 的示例。
 *
 * **加载期校验以 Rust 侧为准**（`src-tauri/src/sources/validate.rs`）：
 * 这里只是给写配置的人看的说明书，不参与运行时判断。
 * 面向用户的完整字段表与排错见 `docs/CUSTOM-SOURCES.md`。
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
   *
   * 写了不认识的占位符（`{pages}`）会在加载阶段被拒：`Url::parse` 会把它编码成
   * 一个"合法但服务端看不懂"的地址，那种 bug 只表现为"结果为空"。
   */
  url: string;
  /** 只实现 GET。写别的会在加载阶段被拒，而不是等运行时才报错。 */
  method?: "GET";
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
  /**
   * 标签字段。
   *
   * **目前是死配置**：格式里有、`ImageRef` 里没有、代码不读、界面不显示。
   * 写了不会报错，但也不会有效果——别以为它能做二次过滤。
   */
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
   * 只写裸参数，不要带 `?` 或 `&`。
   *
   * **和 `sfwSafeEndpoint` 是二选一，必填**：两者都没有的图源在加载阶段被拒。
   * 以前这条只有 code review 能保证；从 v0.2.0 起配置由用户写，必须由代码问出来。
   */
  sfwQuery?: string;
  /**
   * 声明"这个接口本身只提供安全内容"，用于**没有分级参数可配**的图源。
   *
   * 两种正当情形：站点本身没有分级概念（360、nekos.best）；
   * 或者安全条件已经硬编码在地址模板里（safebooru 的 `rating:general`）。
   *
   * 写它等于向以后的维护者承诺"这个接口不会返回越界内容"，
   * 依据要写进文档（`docs/SOURCES-CN.md` §11 是内置 5 个的逐条依据）。
   *
   * **有参数就必须用参数**：wallhaven 有 `purity`、lolicon 有 `r18`，
   * 用一句声明代替就是撒谎。
   */
  sfwSafeEndpoint?: boolean;
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
  /**
   * 精确标签搜到 0 条时，给标签补一个结尾 `*` 再试一次。
   *
   * booru 的标签是"全站用户约定"的，和口语差得很远：`lycoris` 不是标签，
   * `lycoris_recoil` 才是；`elaina` 不存在，`elaina_(majo_no_tabitabi)` 才是。
   * Gelbooru 系的 `tags` 支持 `前缀*`，补一个星号就能把这些词救回来。
   *
   * **只在精确搜到 0 条时发生**，而且界面必须写明"这是前缀扩展的结果"：
   * 前缀匹配是模糊的，会带出名字以它开头的别的东西。
   * 不认 `*` 的接口别开这个（会白发一次请求）。
   */
  wildcardFallback?: boolean;
}

/** 单个图源。 */
export interface WallpaperSource {
  /** 稳定标识，用于缓存目录名和用户配置引用；发布后不要改。 */
  id: string;
  /** 界面上显示的名字。 */
  name: string;
  /**
   * 是否出现在下拉框里。
   *
   * **默认 `true`**（v0.2.0 起）。写给"关掉但不删"的场景：`false` = 加载、但列表里不显示。
   * 默认改过是因为反过来（默认 false）会给用户一个"文件放进去了却看不见、还不报错"的坑。
   */
  enabled?: boolean;
  /** 站点主页，用于在界面上标注来源。 */
  homepage?: string;
  /**
   * 内容提示，比如"尺度较大"。有值就显示在图源名旁边。
   *
   * 这是**提示**不是过滤器——本项目没有机器判定的能力分级，
   * 别把它当成"配了就等于过滤掉了"。
   */
  contentNote?: string;
  request: HttpRequestSpec;
  response: ResponseMapping;
  /** 不配 = 没有内容安全声明 = 加载阶段被拒。见 `SourceFilter.sfwQuery`。 */
  filter?: SourceFilter;
  /** 不配 = 不支持搜索。 */
  search?: SearchSpec;
  /**
   * 来自用户配置目录，不是随程序打包的内置图源。
   *
   * 由加载器填写，**配置文件里写它没有意义**（会被覆盖）。
   */
  custom?: boolean;
}

/**
 * 顶层结构。
 *
 * 以 `_` 或 `$` 开头的键会被忽略，可以当注释用（JSON 没有注释）：
 * `"_comment": "说明"`。
 */
export interface SourcesFile {
  schemaVersion: 1;
  sources: WallpaperSource[];
}
