import { useEffect, useMemo, useState } from 'react'
import type { ModuleProps } from '../types'
import { Notice, RawView, safeJson, useArtifact } from '../shared'
import { MathMarkdown } from '../mathMarkdown'
import { loadLearnerState, updateLearnerState } from '../learnerState'
import { useAssistantI18n } from '../../i18n'
import styles from '../../assistant.module.css'

interface ReviewMistake {
  question_id?: string
  question?: string
  your_answer?: string
  correct_answer?: string
  cause?: string
  approach?: string
  concept?: string
}

interface ReviewCard {
  id?: string
  front?: string
  back?: string
  concept?: string
  due?: string
}

interface ReviewFile {
  summary?: string
  score?: number
  total?: number
  accuracy?: number
  elapsed_seconds?: number
  time_limit_minutes?: number
  weak_concepts?: string[]
  mistakes?: ReviewMistake[]
  cards?: ReviewCard[]
  recommendations?: string[]
}

function reviewClock(seconds?: number): string {
  if (seconds == null) return '—'
  const total = Math.max(0, Math.floor(seconds))
  const m = Math.floor(total / 60)
  const s = total % 60
  return `${m}:${String(s).padStart(2, '0')}`
}

function parseReview(content: string): ReviewFile | null {
  const parsed = safeJson(content)
  return parsed ? (parsed as ReviewFile) : null
}

export function ReviewModule({ projectId, plugin, runId, run }: ModuleProps) {
  const { t, artifactLabel } = useAssistantI18n()
  const artifact = run.artifacts?.find((item) => item.id === 'review') ?? null
  const pluginId = plugin?.id ?? ''
  const { content, error } = useArtifact(
    projectId,
    pluginId,
    runId,
    'review',
    artifact,
  )
  const [ratings, setRatings] = useState<Record<string, 'known' | 'unknown'>>({})
  const [index, setIndex] = useState(0)
  const [flipped, setFlipped] = useState(false)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    let cancelled = false
    loadLearnerState(projectId, pluginId, runId).then((state) => {
      if (!cancelled) setRatings(state.card_ratings ?? {})
    })
    return () => {
      cancelled = true
    }
  }, [projectId, runId])

  const review = content ? parseReview(content) : null
  const cards = review?.cards ?? []
  const mistakes = review?.mistakes ?? []
  const accuracyPct =
    review?.accuracy != null
      ? Math.round(
          Math.min(1, Math.max(0, review.accuracy <= 1 ? review.accuracy : review.accuracy / 100)) *
            100,
        )
      : null
  const known = useMemo(
    () => cards.filter((card, cardIndex) => ratings[card.id ?? `c${cardIndex}`] === 'known').length,
    [cards, ratings],
  )

  if (error) return <div className={styles.error}>{error}</div>
  if (!content) {
    return (
      <section className={styles.panel}>
        <h2>{artifactLabel('review')}</h2>
        <Notice>{t('reviewEmpty')}</Notice>
      </section>
    )
  }
  if (!review) return <RawView title={artifactLabel('review')} content={content} />

  const card = cards[index]
  const cardKey = card ? (card.id ?? `c${index}`) : ''
  const rating = card ? ratings[cardKey] : undefined

  const rate = async (value: 'known' | 'unknown') => {
    if (!card) return
    const next = { ...ratings, [cardKey]: value }
    setRatings(next)
    setSaving(true)
    try {
      await updateLearnerState(projectId, pluginId, runId, { card_ratings: next })
      if (index < cards.length - 1) {
        setIndex(index + 1)
        setFlipped(false)
      }
    } finally {
      setSaving(false)
    }
  }

  return (
    <>
      <section className={styles.panel}>
        <h2>{t('reviewSummary')}</h2>
        {review.summary && (
          <div className={styles.muted}>
            <MathMarkdown text={review.summary} />
          </div>
        )}
        <div className={styles.practiceSummary}>
          <div className={styles.metric}>
            <div className={styles.metricLabel}>{t('reviewScoreLabel')}</div>
            <div className={styles.metricValue}>
              {review.score ?? '—'}/{review.total ?? '—'}
            </div>
          </div>
          <div className={styles.metric}>
            <div className={styles.metricLabel}>{t('reviewAccuracyLabel')}</div>
            <div className={styles.metricValue}>
              {accuracyPct != null ? `${accuracyPct}%` : '—'}
            </div>
          </div>
          <div className={styles.metric}>
            <div className={styles.metricLabel}>{t('reviewTimeLabel')}</div>
            <div className={styles.metricValue}>
              {reviewClock(review.elapsed_seconds)}
            </div>
          </div>
        </div>
        {(review.weak_concepts ?? []).length > 0 && (
          <div className={styles.tagRow}>
            {review.weak_concepts?.map((concept) => (
              <span key={concept} className={styles.tag}>
                <MathMarkdown text={concept} inline />
              </span>
            ))}
          </div>
        )}
        {(review.recommendations ?? []).length > 0 && (
          <div className={styles.recommendations}>
            <strong>{t('reviewRecommendations')}</strong>
            <ul>
              {review.recommendations?.map((item, index) => (
                <li key={index}>
                  <MathMarkdown text={item} inline />
                </li>
              ))}
            </ul>
          </div>
        )}
      </section>

      <section className={styles.panel}>
        <h2>{t('reviewMistakes')}</h2>
        {mistakes.length === 0 ? (
          <Notice>{t('reviewNoMistakes')}</Notice>
        ) : (
          <div className={styles.mistakeList}>
            {mistakes.map((mistake, mistakeIndex) => (
              <details key={mistake.question_id ?? mistakeIndex} className={styles.mistake}>
                <summary>
                  <span>
                    <MathMarkdown
                      text={mistake.question || `#${mistakeIndex + 1}`}
                      inline
                    />
                  </span>
                  {mistake.concept && (
                    <span className={styles.tag}>
                      <MathMarkdown text={mistake.concept} inline />
                    </span>
                  )}
                </summary>
                <div className={styles.mistakeBody}>
                  <div className={styles.answerGrid}>
                    <div>
                      <span className={styles.muted}>{t('reviewYourAnswer')}</span>
                      <MathMarkdown text={mistake.your_answer || '—'} />
                    </div>
                    <div>
                      <span className={styles.muted}>{t('reviewCorrectAnswer')}</span>
                      <MathMarkdown text={mistake.correct_answer || '—'} />
                    </div>
                  </div>
                  {mistake.cause && (
                    <p>
                      <strong>{t('reviewCause')}</strong>{' '}
                      <MathMarkdown text={mistake.cause} inline />
                    </p>
                  )}
                  {mistake.approach && (
                    <p>
                      <strong>{t('reviewApproach')}</strong>{' '}
                      <MathMarkdown text={mistake.approach} inline />
                    </p>
                  )}
                </div>
              </details>
            ))}
          </div>
        )}
      </section>

      <section className={styles.panel}>
        <div className={styles.quizHead}>
          <h2>{t('reviewCards')}</h2>
          <span className={styles.mono}>
            {known}/{cards.length}
          </span>
        </div>
        {cards.length === 0 ? (
          <Notice>{t('reviewNoCards')}</Notice>
        ) : (
          <>
            <div className={styles.flashMeta}>
              <span>
                {t('reviewCardProgress', {
                  current: String(index + 1),
                  total: String(cards.length),
                })}
              </span>
              {card.concept && (
                <span className={styles.tag}>
                  <MathMarkdown text={card.concept} inline />
                </span>
              )}
              {rating && (
                <span className={rating === 'known' ? styles.pillDone : styles.pillGate}>
                  {rating === 'known' ? t('reviewRatedKnown') : t('reviewRatedUnknown')}
                </span>
              )}
            </div>
            <button
              type="button"
              className={`${styles.flashCard} ${flipped ? styles.flashFlipped : ''}`}
              onClick={() => setFlipped((value) => !value)}
            >
              <span className={styles.flashLabel}>
                {flipped ? t('reviewBack') : t('reviewFront')}
              </span>
              <span className={styles.flashText}>
                <MathMarkdown
                  text={flipped ? card.back || '—' : card.front || '—'}
                  inline
                />
              </span>
              <span className={styles.flashHint}>{t('reviewFlip')}</span>
            </button>
            <div className={styles.flashActions}>
              <button
                type="button"
                className={styles.btn}
                disabled={index === 0}
                onClick={() => {
                  setIndex((value) => Math.max(0, value - 1))
                  setFlipped(false)
                }}
              >
                {t('reviewPrev')}
              </button>
              <button
                type="button"
                className={styles.btn}
                disabled={index >= cards.length - 1}
                onClick={() => {
                  setIndex((value) => Math.min(cards.length - 1, value + 1))
                  setFlipped(false)
                }}
              >
                {t('reviewNext')}
              </button>
              <button
                type="button"
                className={styles.btn}
                disabled={saving}
                onClick={() => void rate('unknown')}
              >
                {t('reviewUnknown')}
              </button>
              <button
                type="button"
                className={styles.primary}
                disabled={saving}
                onClick={() => void rate('known')}
              >
                {t('reviewKnown')}
              </button>
            </div>
          </>
        )}
      </section>
    </>
  )
}
