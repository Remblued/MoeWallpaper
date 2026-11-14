//! 加载图源配置：内置那一份 + 用户配置目录里的那些。
//!
//! `v0.1.x` 只读 `include_str!` 进来的内置配置。`v0.2.0` 起多一个来源：
//! `%APPDATA%\com.remblued.moewallpaper\sources\*.json`（见 `fs::paths::sources_dir`）。
//!
//! 三条规矩，每条都是为了一种具体的失望：
//!
//! 1. **一个坏文件不影响别的文件**，更不影响程序启动。用户手写 JSON 一定会写错，
//!    写错的那一刻最需要的不是"程序打不开"，而是"哪一行、哪个字段、怎么改"。
//! 2. **内置排前面，用户文件按文件名排序**。顺序决定下拉框顺序，而"每次打开
//!    顺序都不一样"会让用户以为选中项在乱跳。
//! 3. **id 全局唯一**。撞了 id 的话后加载的那条会被丢掉并报错——不能让它悄悄
//!    覆盖一个能用的图源，否则"我的配置生效了吗"永远说不清。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::sources::model::{SourcesFile, WallpaperSource};
use crate::sources::validate::{self, Checked, Problem};

const BUILTIN: &str = include_str!("../../../sources/builtin/sources.json");

/// 「生成示例配置」写出去的模板。
///
/// **和 `sources/examples/example.sources.json` 是同一个文件**（`include_str!`），
/// 所以文档里说的、测试里验的、用户拿到的是同一份东西。
const EXAMPLE: &str = include_str!("../../../sources/examples/example.sources.json");

/// 内置图源在问题列表里显示成什么。
pub const BUILTIN_LABEL: &str = "内置图源";

/// 目录级问题（读不了目录）挂在哪个"文件"名下。
pub const FOLDER_LABEL: &str = "图源目录";

/// 用户配置目录里的文件名。
pub const EXAMPLE_FILE_NAME: &str = "example.json";

pub fn load_builtin() -> Result<SourcesFile, String> {
    serde_json::from_str(BUILTIN).map_err(|e| format!("解析内置图源配置失败: {}", e))
}

pub fn example_template() -> &'static str {
    EXAMPLE
}

/// 一次加载的全部结果。
pub struct Loaded {
    /// 已通过校验的图源：内置在前，用户文件按文件名排在后面。
    pub sources: Vec<WallpaperSource>,
    /// 所有问题（错误 + 警告）。空 = 一切正常。
    pub problems: Vec<Problem>,
    /// 用户配置目录（即使不存在也返回路径，界面要用它显示"放这里"）。
    pub folder: PathBuf,
    /// 读到的用户配置文件数（坏文件也算，它确实存在）。
    pub user_files: usize,
}

impl Loaded {
    pub fn error_count(&self) -> usize {
        self.problems.iter().filter(|p| p.is_error()).count()
    }

    pub fn user_source_count(&self) -> usize {
        self.sources.iter().filter(|s| s.custom).count()
    }
}

/// 加载内置 + 用户目录。
///
/// **故意不返回 `Result`**：用户配置无论多烂，都只该变成问题列表里的一行字。
/// 唯一能失败的是内置配置（打包事故），那时也要把用户图源继续加载出来——
/// 让用户因为我们的错而一个图源都用不了，是更差的结局。
pub fn load_all(user_dir: &Path) -> Loaded {
    let mut sources: Vec<WallpaperSource> = Vec::new();
    let mut problems: Vec<Problem> = Vec::new();
    // id → 来源，用来查重。BTreeMap 只是为了报错时顺序稳定。
    let mut seen: BTreeMap<String, String> = BTreeMap::new();

    // ── 内置 ──────────────────────────────────────────────
    match load_builtin() {
        Ok(file) => {
            let checked = validate::check_sources_file(&file, BUILTIN_LABEL, true);
            problems.extend(checked.iter().cloned());

            for mut source in file.sources {
                source.custom = false;
                if validate::has_error_for(&problems, &source.id) {
                    // 内置图源也是这样：校验不过就不加载。放它进来等于让用户
                    // 拿着一个明明有问题、却没有任何提示的图源。
                    continue;
                }
                if let Some(previous) = seen.insert(source.id.clone(), BUILTIN_LABEL.to_string()) {
                    problems.push(Problem::error(
                        BUILTIN_LABEL,
                        Some(&source.id),
                        format!("id 和{previous}重复——这是发版事故，请到 GitHub 报 issue"),
                    ));
                    continue;
                }
                sources.push(source);
            }
        }
        Err(e) => problems.push(Problem::error(BUILTIN_LABEL, None, e)),
    }

    // ── 用户目录 ──────────────────────────────────────────
    let files = user_files(user_dir, &mut problems);

    for (name, path) in &files {
        match read_user_file(path) {
            Ok(checked) => absorb(&checked, name, &mut sources, &mut problems, &mut seen),
            Err(message) => problems.push(Problem::error(name, None, message)),
        }
    }

    Loaded {
        sources,
        problems,
        folder: user_dir.to_path_buf(),
        user_files: files.len(),
    }
}

/// 把一份校验过的配置合进来。
fn absorb(
    checked: &Checked,
    file: &str,
    sources: &mut Vec<WallpaperSource>,
    problems: &mut Vec<Problem>,
    seen: &mut BTreeMap<String, String>,
) {
    problems.extend(checked.problems.iter().cloned());

    // 文件级错误（语法 / 版本 / 类型）：整份不生效，这一条已经记进问题了。
    if checked.file_is_broken() {
        return;
    }
    let Some(spec) = &checked.parsed else {
        return;
    };

    // 逐条：有 error 的那条丢掉，其余照常加载。
    // 这样"两个图源里坏了一个"不会连累另一个——那是最让人放弃的一种死法。
    for source in &spec.sources {
        if validate::has_error_for(&checked.problems, &source.id) {
            continue;
        }

        if let Some(previous) = seen.get(&source.id) {
            problems.push(Problem::error(
                file,
                Some(&source.id),
                format!(
                    "id \"{}\" 已经被{previous}占用，这条被跳过了——换个 id",
                    source.id
                ),
            ));
            continue;
        }

        seen.insert(source.id.clone(), file.to_string());

        let mut source = cloned(source);
        source.custom = true;
        sources.push(source);
    }
}

/// `WallpaperSource` 没实现 `Clone`（配置是搬来搬去的，加了会掩盖多余的复制）。
/// 这里需要克隆，是因为校验要留有完整副本、报错要能指向原 id。
/// 用序列化绕一圈比给结构体挂 `Clone` 更符合本项目的取向：**这份数据只在加载时
/// 走一次**，不追求性能，但求不把 `Clone` 变成全局方便。
fn cloned(source: &WallpaperSource) -> WallpaperSource {
    serde_json::from_value(serde_json::to_value(source).expect("自己序列化自己不会失败"))
        .expect("自己序列化出来的东西一定能解析回去")
}

/// 读一个用户配置文件并校验。
///
/// 返回值只有两种：校验结果，或者"连读都读不了"的一句话。
fn read_user_file(path: &Path) -> Result<Checked, String> {
    let name = file_label(path);

    let bytes = std::fs::read(path).map_err(|e| format!("读不出来：{e}"))?;

    // BOM 必须先剥掉：Windows 记事本存"UTF-8 with BOM"很常见，
    // 而 serde_json 会把 BOM 当成非法字符，报一句"expected value at line 1 column 1"，
    // 那句话对用户毫无意义。
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);

    let text = String::from_utf8(bytes.to_vec())
        .map_err(|_| "不是 UTF-8 编码——记事本里的「ANSI」就是 GBK，请另存为 UTF-8".to_string())?;

    Ok(validate::check_file_text(&text, &name))
}

/// 用户目录里要加载的文件，按文件名排序。
///
/// 只认 `.json`（大小写不敏感），跳过 `.` 开头的隐藏文件。
/// 目录不存在不算问题：第一次运行时它还没建。
fn user_files(dir: &Path, problems: &mut Vec<Problem>) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        if dir.exists() {
            problems.push(Problem::warning(
                FOLDER_LABEL,
                None,
                format!("读不了图源目录 {}——检查权限", dir.display()),
            ));
        }
        return Vec::new();
    };

    let mut files: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().is_file())
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            !name.starts_with('.')
                && Path::new(name.as_ref())
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        })
        .map(|e| (file_label(&e.path()), e.path()))
        .collect();

    files.sort_by(|a, b| {
        a.0.to_lowercase()
            .cmp(&b.0.to_lowercase())
            .then_with(|| a.0.cmp(&b.0))
    });
    files
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// 把用户想导入的文件名收拾成"能安全写进目录的名字"。
///
/// 三条要求：不能带目录（`../x.json` 会写到目录外面）、必须以 `.json` 结尾、
/// 不能太长。中文保留——用户的文件本来就叫「我的图源.json」。
pub fn sanitize_file_name(raw: &str) -> String {
    // `Path::file_name` 在 Windows 上两种分隔符都认。
    let name = Path::new(raw)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let stem = name
        .strip_suffix(".json")
        .or_else(|| name.strip_suffix(".JSON"))
        .unwrap_or(&name);

    let cleaned: String = stem
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();

    let cleaned = cleaned.trim_matches(['.', '_', '-']).to_string();
    let cleaned = if cleaned.is_empty() {
        "imported".to_string()
    } else {
        cleaned
    };

    // 60 已经是"文件资源管理器里不会看不见扩展名"的量级
    let cleaned: String = cleaned.chars().take(60).collect();

    format!("{cleaned}.json")
}

/// 把一份配置文本写进用户目录。**写之前必须先过 `check_file_text`**，
/// 这里只做"名字合法 + 原子写"。
pub fn write_user_file(dir: &Path, file_name: &str, text: &str) -> Result<PathBuf, String> {
    let safe = sanitize_file_name(file_name);
    let dest = dir.join(&safe);

    crate::fs::dirs::ensure_dir(dir)?;
    crate::fs::files::write_atomic(&dest, text.as_bytes())?;

    Ok(dest)
}

/// 生成示例配置。已存在就**不覆盖**：用户可能已经照着它改过了，
/// 一次误点把人家改好的文件冲掉是不可接受的。
pub fn create_example(dir: &Path) -> Result<(PathBuf, bool), String> {
    let dest = dir.join(EXAMPLE_FILE_NAME);
    if dest.is_file() {
        return Ok((dest, false));
    }

    crate::fs::dirs::ensure_dir(dir)?;
    crate::fs::files::write_atomic(&dest, EXAMPLE.as_bytes())?;
    Ok((dest, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU32, Ordering};

    /// 造一个测试用的临时目录。不引 `tempfile`：一个 dev-dependency 只为省这十行不值得。
    fn temp_dir(tag: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "moewallpaper-sources-test-{}-{tag}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn demo_source(id: &str, name: &str) -> String {
        format!(
            r#"{{
                "id": "{id}",
                "name": "{name}",
                "request": {{ "url": "https://a.invalid/list?page={{page}}" }},
                "response": {{ "imageUrl": "url" }},
                "filter": {{ "sfwSafeEndpoint": true }}
            }}"#
        )
    }

    fn file_with(names: &[(&str, &str)]) -> String {
        let sources: Vec<String> = names
            .iter()
            .map(|(id, name)| demo_source(id, name))
            .collect();
        format!(
            r#"{{ "schemaVersion": 1, "sources": [{}] }}"#,
            sources.join(",")
        )
    }

    #[test]
    fn builtin_sources_parse() {
        let file = load_builtin().unwrap();

        assert_eq!(file.schema_version, 1);
        assert!(
            !file.sources.is_empty(),
            "一个图源都没有，应用打开就是空的下拉框"
        );
    }

    /// 每个内置图源都必须具备"能真的拉到图"的最低条件。
    #[test]
    fn every_builtin_source_is_usable() {
        let file = load_builtin().unwrap();

        for source in &file.sources {
            assert!(!source.id.is_empty(), "有图源没有 id");
            assert!(
                !source.name.is_empty(),
                "图源 {} 没有名字，下拉框里会显示空白",
                source.id
            );
            assert!(
                source.request.url.starts_with("https://")
                    || source.request.url.starts_with("http://"),
                "图源 {} 的地址不是 http(s)：{}",
                source.id,
                source.request.url
            );
            assert!(
                !source.response.image_url.is_empty(),
                "图源 {} 没配 imageUrl，拉到数据也取不出图片",
                source.id
            );
        }
    }

    /// id 必须唯一：它是缓存目录名、前端选中值、开关持久化的依据。
    #[test]
    fn builtin_source_ids_are_unique() {
        let file = load_builtin().unwrap();

        let mut ids: Vec<&str> = file.sources.iter().map(|s| s.id.as_str()).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), total, "有重复的图源 id");
    }

    /// 配了搜索的图源，`search.url` 里必须有 `{query}`。
    ///
    /// 没有的话关键词会被**静默丢掉**——用户搜"风景"，拿到的是随机图。
    /// 这条测试就是防它的。
    #[test]
    fn search_urls_actually_use_the_keyword() {
        let file = load_builtin().unwrap();

        for source in &file.sources {
            let Some(spec) = &source.search else {
                continue;
            };

            assert!(
                spec.url.contains("{query}"),
                "图源 {} 的 search.url 里没有 {{query}}：{}",
                source.id,
                spec.url
            );
            assert!(
                spec.url.starts_with("https://") || spec.url.starts_with("http://"),
                "图源 {} 的 search.url 不是 http(s)：{}",
                source.id,
                spec.url
            );
        }
    }

    /// 内置图源必须自己先过一遍 v0.2.0 的校验，**一条 error 都不许有**。
    ///
    /// 这条是 `sfwQuery` 那笔债的收尾：从今往后"没声明内容安全条件"是加载期
    /// 硬校验，所以内置的那 5 份必须自己先合法，不能靠"用户看不出来"。
    #[test]
    fn builtin_sources_pass_the_new_validation() {
        let file = load_builtin().unwrap();
        let problems = validate::check_sources_file(&file, BUILTIN_LABEL, true);
        let errors: Vec<&Problem> = problems.iter().filter(|p| p.is_error()).collect();

        assert!(
            errors.is_empty(),
            "内置图源没通过校验：{:?}",
            errors.iter().map(|p| &p.message).collect::<Vec<_>>()
        );
    }

    /// safebooru 声明的是 `sfwSafeEndpoint`（它没有可配的分级参数，
    /// `rating:general` 是**硬编码在地址模板里**的）。
    ///
    /// 那么"模板里必须有 rating:general"这件事就必须有测试守着：
    /// 哪天有人清理 URL 把那一截删了，字段还在、内容限制没了，而校验照样通过。
    #[test]
    fn safebooru_keeps_its_rating_filter_in_the_template() {
        let file = load_builtin().unwrap();
        let source = file
            .sources
            .iter()
            .find(|s| s.id == "safebooru")
            .expect("safebooru 是内置图源");

        assert!(
            source.filter.sfw_query.is_none(),
            "safebooru 的安全条件在模板里，不是 sfwQuery"
        );
        assert!(
            source.filter.sfw_safe_endpoint,
            "safebooru 必须声明 sfwSafeEndpoint，否则它怎么保证安全？"
        );

        assert!(
            source.request.url.contains("rating:general"),
            "浏览模板里必须有 rating:general：{}",
            source.request.url
        );
        let search = source.search.as_ref().expect("safebooru 支持搜索");
        assert!(
            search.url.contains("rating:general"),
            "搜索模板里必须有 rating:general：{}",
            search.url
        );
    }

    /// 示例模板必须**零问题**（连警告都不要）。
    ///
    /// 一个会触发警告的模板，等于告诉用户"照抄也有问题"。
    #[test]
    fn the_example_template_is_clean_and_working() {
        let checked = validate::check_file_text(example_template(), EXAMPLE_FILE_NAME);

        assert!(
            checked.problems.is_empty(),
            "示例模板有问题：{:?}",
            checked
                .problems
                .iter()
                .map(|p| format!("[{:?}] {}", p.severity, p.message))
                .collect::<Vec<_>>()
        );
        assert!(checked.parsed.is_some());
    }

    /// 示例模板写出去之后能被加载器原样读回来（文件名、BOM、编码都对）。
    #[test]
    fn the_example_round_trips_through_the_loader() {
        let dir = temp_dir("example");

        let (path, created) = create_example(&dir).unwrap();
        assert!(created);
        assert_eq!(path.file_name().unwrap(), EXAMPLE_FILE_NAME);

        // 再点一次不能覆盖
        let (_, created_again) = create_example(&dir).unwrap();
        assert!(!created_again, "已存在的示例文件不该被覆盖");

        let loaded = load_all(&dir);
        assert_eq!(loaded.user_files, 1);
        assert!(
            loaded.problems.iter().all(|p| !p.is_error()),
            "{:?}",
            loaded.problems
        );
        assert!(
            loaded.sources.iter().any(|s| s.custom && s.id == "example"),
            "示例应该作为自定义图源加载进来（enabled:false 只是不显示）"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 用户文件排在全部内置图源后面。
    #[test]
    fn user_sources_come_after_builtin_ones() {
        let dir = temp_dir("order");
        std::fs::write(dir.join("a.json"), file_with(&[("mine", "我的")])).unwrap();

        let loaded = load_all(&dir);
        let builtin_count = load_builtin().unwrap().sources.len();

        assert_eq!(loaded.sources.len(), builtin_count + 1);
        assert!(!loaded.sources[0].custom, "第一个必须还是内置图源");
        let last = loaded.sources.last().unwrap();
        assert_eq!(last.id, "mine");
        assert!(last.custom, "用户图源要打上 custom 标记");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **一个坏文件不能影响别的文件，更不能让加载失败。**
    ///
    /// 这是整个模块最重要的性质：用户写错 JSON 是常态，
    /// 而"写错一个字符 → 整个应用一个图源都没有"会让人直接卸载。
    #[test]
    fn a_broken_file_does_not_take_the_others_down() {
        let dir = temp_dir("broken");
        std::fs::write(dir.join("good.json"), file_with(&[("good", "好的")])).unwrap();
        std::fs::write(dir.join("bad.json"), "{ \"schemaVersion\": 1, ").unwrap();
        std::fs::write(
            dir.join("gbk.json"),
            // 一个不合法的 UTF-8 序列，模拟记事本存成 ANSI
            [0xC4, 0xE3, 0xBA, 0xC3, 0x0A],
        )
        .unwrap();

        let loaded = load_all(&dir);

        assert!(
            loaded.sources.iter().any(|s| s.id == "good"),
            "好文件必须照常加载"
        );
        assert!(
            !loaded
                .sources
                .iter()
                .any(|s| s.id == "bad" || s.id == "gbk"),
            "坏文件里的东西不该出现"
        );
        assert!(loaded.error_count() >= 2, "{:?}", loaded.problems);

        let messages: Vec<&str> = loaded.problems.iter().map(|p| p.message.as_str()).collect();
        assert!(
            messages.iter().any(|m| m.contains("JSON 语法错误")),
            "{messages:?}"
        );
        assert!(
            messages.iter().any(|m| m.contains("UTF-8")),
            "非 UTF-8 的提示必须告诉用户怎么救：{messages:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 一份文件里两条图源、坏了一条：坏的那条丢掉，好的那条照常出现。
    #[test]
    fn one_bad_source_does_not_drop_its_neighbours() {
        let dir = temp_dir("mixed");
        std::fs::write(
            dir.join("two.json"),
            r#"{
                "schemaVersion": 1,
                "sources": [
                    {
                        "id": "ok-one",
                        "name": "好的",
                        "request": { "url": "https://a.invalid/l?page={page}" },
                        "response": { "imageUrl": "url" },
                        "filter": { "sfwSafeEndpoint": true }
                    },
                    {
                        "id": "no-sfw",
                        "name": "忘了写安全声明",
                        "request": { "url": "https://a.invalid/l?page={page}" },
                        "response": { "imageUrl": "url" }
                    }
                ]
            }"#,
        )
        .unwrap();

        let loaded = load_all(&dir);

        assert!(loaded.sources.iter().any(|s| s.id == "ok-one"));
        assert!(
            !loaded.sources.iter().any(|s| s.id == "no-sfw"),
            "没写安全声明的图源必须被拒——这就是 v0.2.0 的硬校验"
        );
        assert_eq!(
            loaded.error_count(),
            1,
            "{:?}",
            loaded
                .problems
                .iter()
                .map(|p| (&p.message, p.severity))
                .collect::<Vec<_>>()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 撞 id：内部图源赢，用户那条被跳过，并且**报出来**。
    ///
    /// 悄悄覆盖是最坏的：用户以为"我的配置生效了"，其实用的是我们的。
    #[test]
    fn a_duplicate_id_is_reported_and_skipped() {
        let dir = temp_dir("dup");
        std::fs::write(dir.join("dup.json"), file_with(&[("wallhaven", "假的")])).unwrap();

        let loaded = load_all(&dir);

        let wallhaven: Vec<&WallpaperSource> = loaded
            .sources
            .iter()
            .filter(|s| s.id == "wallhaven")
            .collect();
        assert_eq!(wallhaven.len(), 1, "同一个 id 只能出现一次");
        assert!(!wallhaven[0].custom, "活下来的应该是内置那条");

        assert!(
            loaded
                .problems
                .iter()
                .any(|p| p.is_error() && p.message.contains("已经被"))
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 同一份文件里自己撞自己也算。
    #[test]
    fn duplicate_ids_inside_one_file_are_reported() {
        let dir = temp_dir("dup2");
        std::fs::write(
            dir.join("twice.json"),
            file_with(&[("same", "一个"), ("same", "两个")]),
        )
        .unwrap();

        let loaded = load_all(&dir);
        assert_eq!(loaded.sources.iter().filter(|s| s.id == "same").count(), 1);
        assert!(loaded.problems.iter().any(|p| p.message.contains("已经被")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// BOM 要能吃：记事本存 UTF-8 with BOM 是最常见的一种"看起来没问题"。
    #[test]
    fn a_utf8_bom_is_tolerated() {
        let dir = temp_dir("bom");

        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(file_with(&[("bom", "带 BOM 的")]).as_bytes());
        std::fs::write(dir.join("bom.json"), bytes).unwrap();

        let loaded = load_all(&dir);
        assert!(
            loaded.sources.iter().any(|s| s.id == "bom"),
            "{:?}",
            loaded.problems
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 非 `.json` 一律不读；隐藏文件也不读。
    #[test]
    fn only_json_files_are_read() {
        let dir = temp_dir("ext");
        std::fs::write(dir.join("notes.txt"), file_with(&[("txt", "不算")])).unwrap();
        std::fs::write(dir.join(".hidden.json"), file_with(&[("hidden", "不算")])).unwrap();
        std::fs::write(dir.join("real.JSON"), file_with(&[("upper", "算")])).unwrap();

        let loaded = load_all(&dir);
        let ids: Vec<&str> = loaded.sources.iter().map(|s| s.id.as_str()).collect();

        assert!(ids.contains(&"upper"), "扩展名大小写不该影响");
        assert!(!ids.contains(&"txt"));
        assert!(!ids.contains(&"hidden"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 目录不存在时加载照常成功（第一次运行就是这样）。
    #[test]
    fn a_missing_folder_is_not_an_error() {
        let dir = std::env::temp_dir().join("moewallpaper-sources-never-exists");
        let _ = std::fs::remove_dir_all(&dir);

        let loaded = load_all(&dir);
        assert!(loaded.error_count() == 0, "{:?}", loaded.problems);
        assert!(!loaded.sources.is_empty(), "内置图源总该有");
    }

    #[test]
    fn file_names_are_sanitized_before_writing() {
        assert_eq!(sanitize_file_name("my.json"), "my.json");
        assert_eq!(sanitize_file_name("../escape.json"), "escape.json");
        assert_eq!(sanitize_file_name(r"..\..\escape.json"), "escape.json");
        assert_eq!(sanitize_file_name("我的图源.json"), "我的图源.json");
        assert_eq!(sanitize_file_name("a b.json"), "a_b.json");
        assert_eq!(sanitize_file_name("noext"), "noext.json");
        assert_eq!(sanitize_file_name(".json"), "imported.json");
        assert_eq!(sanitize_file_name(""), "imported.json");
        assert_eq!(sanitize_file_name("CON.json"), "CON.json");

        let long = sanitize_file_name(&"x".repeat(200));
        assert_eq!(long.chars().count(), 65, "60 个字符 + .json");
    }

    /// 导入的文件真的落到目录里，并且能被下一次加载读出来。
    #[test]
    fn an_imported_file_is_loadable() {
        let dir = temp_dir("import");
        let text = file_with(&[("imported", "导入的")]);

        let path = write_user_file(&dir, "../导入的图源.json", &text).unwrap();
        assert_eq!(path.file_name().unwrap(), "导入的图源.json");
        assert!(path.starts_with(&dir), "不能写到目录外面去");

        let loaded = load_all(&dir);
        assert!(loaded.sources.iter().any(|s| s.id == "imported"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 排序要稳定：同一批文件，两次加载的顺序必须一样。
    #[test]
    fn the_order_is_stable() {
        let dir = temp_dir("stable");
        std::fs::write(dir.join("b.json"), file_with(&[("b", "乙")])).unwrap();
        std::fs::write(dir.join("a.json"), file_with(&[("a", "甲")])).unwrap();

        let first: Vec<String> = load_all(&dir).sources.into_iter().map(|s| s.id).collect();
        let second: Vec<String> = load_all(&dir).sources.into_iter().map(|s| s.id).collect();

        assert_eq!(first, second);
        let a = first.iter().position(|id| id == "a").unwrap();
        let b = first.iter().position(|id| id == "b").unwrap();
        assert!(a < b, "按文件名排序：a.json 在 b.json 前面");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 被 `enabled: false` 关掉的图源仍然加载（前端负责不显示），
    /// 因为"加载"和"显示"是两件事，混起来会让开关失效。
    #[test]
    fn disabled_sources_are_still_loaded() {
        let dir = temp_dir("disabled");
        std::fs::write(
            dir.join("off.json"),
            r#"{
                "schemaVersion": 1,
                "sources": [{
                    "id": "off",
                    "name": "关掉的",
                    "enabled": false,
                    "request": { "url": "https://a.invalid/l?page={page}" },
                    "response": { "imageUrl": "url" },
                    "filter": { "sfwSafeEndpoint": true }
                }]
            }"#,
        )
        .unwrap();

        let loaded = load_all(&dir);
        let source = loaded.sources.iter().find(|s| s.id == "off").unwrap();
        assert!(!source.enabled);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 真实联网：**文档里那份示例模板真的能拉到图**。
    ///
    /// 手动跑：`cargo test -- --ignored live_the_example_template_works`
    ///
    /// 为什么值得联网跑一次：模板错了（少一个 `{limit}`、占位符拼错、地址过时）
    /// 的后果是"用户照着抄，还是不行"，而这类问题所有离线测试都照不出来——
    /// 它们只证明"这份 JSON 是合法的"。
    #[tokio::test]
    #[ignore = "会真的联网拉一页图"]
    async fn live_the_example_template_works() {
        let dir = temp_dir("live-example");
        create_example(&dir).unwrap();

        let loaded = load_all(&dir);
        let source = loaded
            .sources
            .iter()
            .find(|s| s.id == "example")
            .expect("示例模板应该作为自定义图源加载进来");
        assert!(source.custom, "它必须来自用户目录");

        // enabled:false 只是"不显示在下拉框里"，和"能不能拉数据"无关。
        let page = crate::sources::fetch::fetch_source_images(source, 1, None)
            .await
            .expect("示例模板的请求应该成功");

        println!("示例模板拉到 {} 张图", page.images.len());
        assert!(
            !page.images.is_empty(),
            "示例模板一张图都没拉到——模板或接口出问题了"
        );
        assert!(
            page.images
                .iter()
                .all(|i| i.image_url.starts_with("https://")),
            "示例模板的图片地址应该是 https"
        );
        assert!(
            page.images.iter().all(|i| i.thumbnail_url.is_some()),
            "示例模板配了 thumbnailUrl，应该每张都有缩略图"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 真实联网：**一份用户自己写的配置**能从加载一直走到拉到图。
    ///
    /// 手动跑：`cargo test -- --ignored live_a_user_source_pulls_images`
    #[tokio::test]
    #[ignore = "会真的联网拉一页图"]
    async fn live_a_user_source_pulls_images() {
        let dir = temp_dir("live-user");
        std::fs::write(
            dir.join("mine.json"),
            r#"{
                "schemaVersion": 1,
                "sources": [{
                    "id": "my-booru",
                    "name": "我自己接的",
                    "request": {
                        "url": "https://safebooru.org/index.php?page=dapi&s=post&q=index&json=1&limit={limit}&pid={pageIndex}&tags=rating:general+width:>=1920",
                        "headers": { "Accept": "application/json" }
                    },
                    "response": {
                        "imageUrl": "file_url",
                        "thumbnailUrl": "preview_url",
                        "width": "width",
                        "height": "height"
                    },
                    "filter": { "randomPageIndexMax": 100, "sfwSafeEndpoint": true }
                }]
            }"#,
        )
        .unwrap();

        let loaded = load_all(&dir);
        assert!(
            loaded.problems.iter().all(|p| !p.is_error()),
            "{:?}",
            loaded.problems
        );

        let source = loaded
            .sources
            .iter()
            .find(|s| s.id == "my-booru")
            .expect("用户图源应该被加载");

        let page = crate::sources::fetch::fetch_source_images(source, 1, None)
            .await
            .expect("用户图源的请求应该成功");

        println!("用户图源拉到 {} 张图", page.images.len());
        assert!(!page.images.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
