import type { PluginModule } from '@agentzoo/web-ui'
import { AssistantDashboard } from './AssistantDashboard'

// The learning assistant's frontend half: the dashboard component mounted by
// the host shell, plus the chat declaration (agent + preset tasks) that
// configures the host's common chat sidebar.
const plugin: PluginModule = {
  id: 'assistant',
  dashboard: AssistantDashboard,
  chat: {
    agent: 'main',
    placeholder: {
      zh: '告诉我考试/主题、考纲或真题，开始限时刷题…',
      en: 'Tell me the exam/topic, syllabus, or past papers, and start timed practice…',
    },
    tasks: [
      {
        id: 'analyze_syllabus',
        label: { zh: '分析考纲与真题', en: 'Analyze syllabus and past papers' },
        zh: '请分析我提供的考纲和往年真题：整理知识点、题型、难度和考点分布，作为出题依据。',
        en: 'Analyze the syllabus and past papers I provided: summarize knowledge points, question types, difficulty, and topic distribution as the basis for a practice set.',
      },
      {
        id: 'timed_practice',
        label: { zh: '开始限时刷题', en: 'Start timed practice' },
        zh: '请先一次一个问题地询问主题、知识点/考纲/往年真题、题量和限时（能选项的用 ask_user 选项）；确认后调用 start_practice 工具创建并启动限时刷题。',
        en: 'Ask me one question at a time for the topic, knowledge points/syllabus/past papers, question count, and time limit (use ask_user options when the answer is a small set); after confirming, call the start_practice tool to create and start the timed practice.',
      },
      {
        id: 'review_mistakes',
        label: { zh: '分析刷题结果', en: 'Analyze practice result' },
        zh: '请根据我的答题结果和用时分析这次刷题：正确率、薄弱知识点、错因分类、时间分配和下一步刷题建议。',
        en: 'Analyze this practice run from my answers and time spent: accuracy, weak knowledge points, mistake causes, time allocation, and what to practice next.',
      },
      {
        id: 'explain_material',
        label: { zh: '讲解知识点', en: 'Explain knowledge points' },
        zh: '请讲解我提供的知识点或错题：给出直觉、例子、易错点和同类题思路。',
        en: 'Explain the knowledge points or mistakes I provided: intuition, examples, common pitfalls, and how to approach similar questions.',
      },
      {
        id: 'learning_report',
        label: { zh: '刷题报告', en: 'Practice report' },
        zh: '请汇总这次刷题结果，生成一份报告：成绩、用时、正确率、薄弱知识点和下一步刷题计划。',
        en: 'Summarize this practice run into a report: score, time spent, accuracy, weak knowledge points, and the next practice plan.',
      },
    ],
  },
}

export default plugin
