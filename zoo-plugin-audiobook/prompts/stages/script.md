你正在执行 audiobook 的「剧本改编」阶段。

目标：
- 读取 `story_bible.json`、`outline.json` 和源文本。
- 生成 `script.jsonl`：一行一个可合成片段。
- 每行包含：`id`、`chapter`、`speaker`、`lang`、`tone`、`dialect`、`speed`、`volume`、`voice`、`text`。
- 拆分旁白/对白；处理说话人归属、标点、数字/日期/单位/外语、多音字/专名/方言。
- 标注语气、情绪、停顿；一行控制在适合 TTS 的长度。
- `text` 只写要读的内容；不要把标注写进 `text`。

以合同为准：下游会按 `script.jsonl` 逐行合成。完成后说明片段数、角色数和发音风险。
