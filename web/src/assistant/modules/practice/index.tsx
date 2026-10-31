import { useEffect, useMemo, useRef, useState } from 'react'
import type { ModuleProps } from '../types'
import { Notice, RawView, safeJson, useArtifact } from '../shared'
import { MathMarkdown } from '../mathMarkdown'
import {
  loadLearnerState,
  updateLearnerState,
  type LearnerAnswer,
} from '../learnerState'
import { useAssistantI18n } from '../../i18n'
import styles from '../../assistant.module.css'

type QuizType = 'choice' | 'short' | 'code'

interface QuizQuestion {
  id?: string
  type?: string
  question?: string
  options?: string[]
  answer?: number | string
  explanation?: string
  difficulty?: string
  concept?: string
  source?: string
}

interface QuizFile {
  title?: string
  time_limit_minutes?: number
  question_count?: number
  instructions?: string
  questions?: QuizQuestion[]
}

function parseQuiz(content: string): QuizFile | null {
  const parsed = safeJson(content)
  if (!parsed) return null
  const quiz = parsed as QuizFile
  if (Array.isArray(quiz.questions)) return quiz
  if (Array.isArray(parsed)) return { questions: parsed as QuizQuestion[] }
  return null
}

function questionType(question: QuizQuestion): QuizType {
  if (question.type === 'choice' || question.type === 'short' || question.type === 'code') {
    return question.type
  }
  return Array.isArray(question.options) ? 'choice' : 'short'
}

function choiceIndex(question: QuizQuestion): number {
  if (typeof question.answer === 'number') return question.answer
  if (typeof question.answer === 'string' && question.options) {
    return question.options.indexOf(question.answer)
  }
  return -1
}

function keyFor(question: QuizQuestion, index: number): string {
  return question.id ?? `q${index + 1}`
}

function clock(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds))
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  return h > 0
    ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
    : `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
}

export function PracticeModule({ projectId, runId, run, onAct }: ModuleProps) {
  const { t, artifactLabel } = useAssistantI18n()
  const artifact = run.artifacts?.find((item) => item.id === 'quiz') ?? null
  const { content, error } = useArtifact(projectId, runId, 'quiz', artifact)
  const [answers, setAnswers] = useState<Record<string, LearnerAnswer>>({})
  const [submitted, setSubmitted] = useState(false)
  const [startedAt, setStartedAt] = useState<string | null>(null)
  const [submittedAt, setSubmittedAt] = useState<string | null>(null)
  const [elapsed, setElapsed] = useState(0)
  const [remaining, setRemaining] = useState<number | null>(null)
  const [autoSubmitted, setAutoSubmitted] = useState(false)
  const [saving, setSaving] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)
  const autoSubmitRef = useRef(false)

  const quiz = content ? parseQuiz(content) : null
  const questions = quiz?.questions ?? []
  const input = run.input as { time_limit_minutes?: unknown } | undefined
  const limitMinutes =
    typeof input?.time_limit_minutes === 'number'
      ? input.time_limit_minutes
      : quiz?.time_limit_minutes ?? 30
  const limitSeconds = Math.max(1, limitMinutes) * 60
  const score = useMemo(
    () => Object.values(answers).filter((answer) => answer.correct === true).length,
    [answers],
  )
  const answered = useMemo(
    () =>
      questions.filter((q, i) => {
        const value = answers[keyFor(q, i)]?.value
        return value !== undefined && value !== ''
      }).length,
    [answers, questions],
  )
  const accuracy = questions.length
    ? Math.round((score / questions.length) * 100)
    : 0
  // Keep every hook above the early returns: the component first renders
  // without the artifact, then re-renders once it loads.
  const conceptStats = useMemo(() => {
    const stats = new Map<string, { correct: number; total: number }>()
    questions.forEach((question, index) => {
      const key = question.concept || '—'
      const answer = answers[keyFor(question, index)]
      const entry = stats.get(key) ?? { correct: 0, total: 0 }
      entry.total += 1
      if (answer?.correct === true) entry.correct += 1
      stats.set(key, entry)
    })
    return [...stats.entries()]
      .filter(([, value]) => value.correct < value.total)
      .sort((a, b) => a[1].correct / a[1].total - b[1].correct / b[1].total)
      .slice(0, 4)
  }, [answers, questions, t])

  useEffect(() => {
    let cancelled = false
    loadLearnerState(projectId, runId).then((state) => {
      if (cancelled) return
      setAnswers(state.answers ?? {})
      setSubmitted(Boolean(state.submitted))
      setStartedAt(state.started_at ?? null)
      setSubmittedAt(state.submitted_at ?? null)
      setElapsed(state.elapsed_seconds ?? 0)
      setAutoSubmitted(Boolean(state.auto_submitted))
      if (state.started_at && !state.submitted) {
        const left =
          (state.time_limit_minutes ?? limitMinutes) * 60 -
          Math.floor((Date.now() - new Date(state.started_at).getTime()) / 1000)
        setRemaining(Math.max(0, left))
      }
    })
    return () => {
      cancelled = true
    }
  }, [projectId, runId, limitMinutes])

  // The countdown is derived from the persisted start timestamp, so reloading
  // the page resumes the same attempt instead of restarting the clock.
  const submitRef = useRef<(auto: boolean) => void>(() => {})
  useEffect(() => {
    if (!startedAt || submitted) return
    const startMs = new Date(startedAt).getTime()
    const tick = () => {
      const left = Math.max(
        0,
        limitSeconds - Math.floor((Date.now() - startMs) / 1000),
      )
      setRemaining(left)
      if (left === 0 && !autoSubmitRef.current) {
        autoSubmitRef.current = true
        submitRef.current(true)
      }
    }
    tick()
    const id = window.setInterval(tick, 1000)
    return () => window.clearInterval(id)
  }, [startedAt, submitted, limitSeconds])

  if (error) return <div className={styles.error}>{error}</div>
  if (!content) {
    return (
      <section className={styles.panel}>
        <h2>{artifactLabel('quiz')}</h2>
        <Notice>{t('practiceEmpty')}</Notice>
      </section>
    )
  }
  if (!quiz) return <RawView title={artifactLabel('quiz')} content={content} />
  if (questions.length === 0) {
    return (
      <section className={styles.panel}>
        <h2>{quiz.title || artifactLabel('quiz')}</h2>
        <Notice>{t('practiceEmpty')}</Notice>
      </section>
    )
  }

  const persist = async (
    nextAnswers: Record<string, LearnerAnswer>,
    nextSubmitted: boolean,
    extra: {
      startedAt?: string | null
      submittedAt?: string | null
      elapsed?: number
      auto?: boolean
    } = {},
  ) => {
    setSaving(true)
    setSaveError(null)
    try {
      const nextStartedAt =
        extra.startedAt === undefined ? startedAt : extra.startedAt
      const nextSubmittedAt =
        extra.submittedAt === undefined ? submittedAt : extra.submittedAt
      await updateLearnerState(projectId, runId, {
        answers: nextAnswers,
        submitted: nextSubmitted,
        score: Object.values(nextAnswers).filter((a) => a.correct === true).length,
        total: questions.length,
        started_at: nextStartedAt,
        submitted_at: nextSubmittedAt,
        elapsed_seconds: extra.elapsed ?? elapsed,
        time_limit_minutes: limitMinutes,
        auto_submitted: extra.auto ?? autoSubmitted,
      })
    } catch (e) {
      setSaveError(String(e))
    } finally {
      setSaving(false)
    }
  }

  const setAnswer = (key: string, value: number | string) => {
    setAnswers((prev) => ({
      ...prev,
      [key]: { ...(prev[key] ?? {}), value },
    }))
  }

  const start = () => {
    const now = new Date().toISOString()
    autoSubmitRef.current = false
    setAnswers({})
    setSubmitted(false)
    setStartedAt(now)
    setSubmittedAt(null)
    setElapsed(0)
    setRemaining(limitSeconds)
    setAutoSubmitted(false)
    void persist({}, false, {
      startedAt: now,
      submittedAt: null,
      elapsed: 0,
      auto: false,
    })
  }

  const submit = (auto = false) => {
    const now = Date.now()
    const startMs = startedAt ? new Date(startedAt).getTime() : now
    const used = Math.min(
      limitSeconds,
      Math.max(0, Math.floor((now - startMs) / 1000)),
    )
    const next: Record<string, LearnerAnswer> = {}
    questions.forEach((question, index) => {
      const key = keyFor(question, index)
      const current = answers[key]
      const value = current?.value ?? ''
      if (questionType(question) === 'choice') {
        next[key] = {
          value,
          correct: typeof value === 'number' && value === choiceIndex(question),
        }
      } else {
        next[key] = {
          value,
          correct: value === '' ? false : current?.correct,
        }
      }
    })
    const at = new Date().toISOString()
    setAnswers(next)
    setSubmitted(true)
    setSubmittedAt(at)
    setElapsed(used)
    setRemaining(0)
    setAutoSubmitted(auto)
    if (!auto) autoSubmitRef.current = false
    void persist(next, true, {
      startedAt,
      submittedAt: at,
      elapsed: used,
      auto,
    })
  }
  submitRef.current = submit

  const reset = () => {
    autoSubmitRef.current = false
    setAnswers({})
    setSubmitted(false)
    setStartedAt(null)
    setSubmittedAt(null)
    setElapsed(0)
    setRemaining(null)
    setAutoSubmitted(false)
    void updateLearnerState(projectId, runId, {
      answers: {},
      submitted: false,
      score: 0,
      total: 0,
      started_at: null,
      submitted_at: null,
      elapsed_seconds: 0,
      auto_submitted: false,
    })
  }

  const mark = (key: string, correct: boolean) => {
    const next = {
      ...answers,
      [key]: { ...(answers[key] ?? { value: '' }), correct },
    }
    setAnswers(next)
    void persist(next, true, { startedAt, submittedAt, elapsed, auto: autoSubmitted })
  }

  const sourceLabel = (source?: string) => {
    if (source === 'syllabus') return t('practiceSourceSyllabus')
    if (source === 'past_paper') return t('practiceSourcePast')
    if (source === 'knowledge_point') return t('practiceSourceKnowledge')
    return ''
  }

  const notStarted = !startedAt && !submitted

  return (
    <section className={styles.panel}>
      <div className={styles.quizHead}>
        <div>
          <h2>{quiz.title || t('practiceStartTitle')}</h2>
          <p className={styles.muted}>
            {t('practiceQuestions', { count: String(questions.length) })} ·{' '}
            {t('practiceTimeLimit', { minutes: String(limitMinutes) })}
          </p>
        </div>
        <div className={styles.quizActions}>
          {notStarted && (
            <button type="button" className={styles.primary} onClick={start}>
              {t('practiceStart')}
            </button>
          )}
          {startedAt && !submitted && (
            <>
              <span className={styles.quizTimer}>
                {t('practiceTimer')} {clock(remaining ?? limitSeconds)}
              </span>
              <button type="button" className={styles.primary} onClick={() => submit(false)}>
                {t('quizSubmit')}
              </button>
            </>
          )}
          {submitted && (
            <button type="button" className={styles.btn} onClick={reset}>
              {t('quizReset')}
            </button>
          )}
        </div>
      </div>

      {notStarted && (
        <div className={styles.practiceIntro}>
          {quiz.instructions && (
            <div className={styles.muted}>
              <MathMarkdown text={quiz.instructions} />
            </div>
          )}
          <p className={styles.muted}>{t('practiceStartHint')}</p>
        </div>
      )}

      {submitted && (
        <div className={styles.practiceSummary}>
          <div className={styles.metric}>
            <div className={styles.metricLabel}>{t('practiceSubmitted')}</div>
            <div className={styles.metricValue}>
              {score}/{questions.length} · {accuracy}%
            </div>
          </div>
          <div className={styles.metric}>
            <div className={styles.metricLabel}>{t('practiceTimeUsed')}</div>
            <div className={styles.metricValue}>{clock(elapsed)}</div>
          </div>
          <div className={styles.metric}>
            <div className={styles.metricLabel}>{t('practiceAccuracyLabel')}</div>
            <div className={styles.metricValue}>{accuracy}%</div>
          </div>
        </div>
      )}
      {submitted && autoSubmitted && (
        <Notice>{t('practiceTimeUp')}</Notice>
      )}
      {submitted && conceptStats.length > 0 && (
        <div className={styles.weakRow}>
          {conceptStats.map(([concept, value]) => (
            <span key={concept} className={styles.tag}>
              {concept} {value.correct}/{value.total}
            </span>
          ))}
        </div>
      )}
      {submitted && (
        <div className={styles.analysisCta}>
          <span className={styles.muted}>{t('practiceAnalyzeHint')}</span>
          {run.status === 'waiting_gate' && (
            <button
              type="button"
              className={styles.primary}
              onClick={() => onAct('approve')}
            >
              {t('practiceAnalyze')}
            </button>
          )}
        </div>
      )}

      {saveError && <div className={styles.error}>{saveError}</div>}
      {saving && <div className={styles.muted}>{t('quizSaving')}</div>}

      {!notStarted && (
        <div className={styles.quizList}>
          {questions.map((question, index) => {
            const key = keyFor(question, index)
            const type = questionType(question)
            const answer = answers[key]
            const correctIndex = choiceIndex(question)
            const isCorrect = answer?.correct === true
            const isWrong = answer?.correct === false
            const source = sourceLabel(question.source)
            const reference =
              typeof question.answer === 'string'
                ? question.answer
                : String(question.answer ?? '')
            return (
              <article key={key} className={styles.quizCard}>
                <div className={styles.quizMeta}>
                  <span className={styles.quizNumber}>#{index + 1}</span>
                  <span className={styles.tag}>
                    {type === 'choice'
                      ? t('quizChoice')
                      : type === 'code'
                        ? t('quizCode')
                        : t('quizShort')}
                  </span>
                  {question.difficulty && (
                    <span className={styles.tag}>{question.difficulty}</span>
                  )}
                  {question.concept && (
                    <span className={styles.tag}>{question.concept}</span>
                  )}
                  {source && (
                    <span className={styles.tag}>
                      {t('practiceSource')}: {source}
                    </span>
                  )}
                  {submitted && (
                    <span className={isCorrect ? styles.pillDone : isWrong ? styles.badgeFail : styles.pill}>
                      {isCorrect ? t('quizCorrect') : isWrong ? t('quizWrong') : t('quizPending')}
                    </span>
                  )}
                </div>
                <div className={styles.quizQuestion}>
                  <MathMarkdown text={question.question ?? ''} />
                </div>

                {type === 'choice' ? (
                  <div className={styles.choiceList}>
                    {(question.options ?? []).map((option, optionIndex) => {
                      const selected = answer?.value === optionIndex
                      const showCorrect = submitted && optionIndex === correctIndex
                      const showWrong = submitted && selected && optionIndex !== correctIndex
                      return (
                        <label
                          key={optionIndex}
                          className={`${styles.choice} ${selected ? styles.choiceSelected : ''} ${showCorrect ? styles.choiceCorrect : ''} ${showWrong ? styles.choiceWrong : ''}`}
                        >
                          <input
                            type="radio"
                            name={key}
                            checked={selected}
                            disabled={submitted}
                            onChange={() => setAnswer(key, optionIndex)}
                          />
                          <span>
                            <MathMarkdown text={option} inline />
                          </span>
                        </label>
                      )
                    })}
                  </div>
                ) : (
                  <textarea
                    className={styles.answerInput}
                    rows={4}
                    value={typeof answer?.value === 'string' ? answer.value : ''}
                    disabled={submitted}
                    placeholder={t('quizPlaceholder')}
                    onChange={(e) => setAnswer(key, e.target.value)}
                  />
                )}

                {submitted && (
                  <div className={styles.quizReveal}>
                    {question.explanation && (
                      <div className={styles.explanation}>
                        <MathMarkdown text={question.explanation} />
                      </div>
                    )}
                    {type !== 'choice' && (
                      <>
                        <div className={styles.reference}>
                          <strong>{t('quizReference')}</strong>
                          {type === 'code' ? (
                            <pre className={styles.pre}>{reference}</pre>
                          ) : (
                            <MathMarkdown text={reference} />
                          )}
                        </div>
                        {answer?.correct == null && (
                          <div className={styles.selfMark}>
                            <span>{t('quizSelfMark')}</span>
                            <button
                              type="button"
                              className={styles.btn}
                              onClick={() => mark(key, true)}
                            >
                              {t('quizMarkCorrect')}
                            </button>
                            <button
                              type="button"
                              className={styles.btn}
                              onClick={() => mark(key, false)}
                            >
                              {t('quizMarkWrong')}
                            </button>
                          </div>
                        )}
                      </>
                    )}
                  </div>
                )}
              </article>
            )
          })}
        </div>
      )}
    </section>
  )
}
