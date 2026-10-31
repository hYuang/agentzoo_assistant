你正在执行 audiobook 的「选题与方向」阶段。用户可能什么都没有，只有一个模糊的愿望。

先读取 `project.json` 里的 `constraints`、`idea`、`outline`、`source` 和制作设定。
- 如果用户已经有明确想法/大纲/原稿：围绕它提炼 1 个主方向，并给出 2 个可选变体。
- 如果用户从 0 开始：生成 5 个差异明显的方向，不要只换标题。

每个方向必须包含：
- `id`, `title`, `premise`（一句到三句核心故事）
- `hook`（为什么听众会点开）
- `genre`, `audience`, `tone`, `length`
- `why_it_works`（冲突、人物欲望、声音表现空间）

写入 `directions.json`。不要写完整故事圣经，也不要写剧本。
完成后说明：哪个方向最适合做成有声书，为什么。
