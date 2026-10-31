import { useCallback, useEffect, useRef, useState } from 'react'
import { BookOpen, Search, Upload } from 'lucide-react'
import type { ModuleProps } from '../types'
import { Notice } from '../shared'
import { MathMarkdown } from '../mathMarkdown'
import { useAssistantI18n } from '../../i18n'
import {
  listKbFiles,
  searchKb,
  uploadKbFile,
} from '@agentzoo/web-ui'
import type { KbFileInfo, KbSearchHitInfo } from '@agentzoo/web-ui'
import styles from '../../assistant.module.css'

const KB_DOC_TYPES = [
  '.md',
  '.markdown',
  '.txt',
  '.pdf',
  '.doc',
  '.docx',
  '.ppt',
  '.pptx',
  '.xls',
  '.xlsx',
]

function formatSize(size: number): string {
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / 1024 / 1024).toFixed(1)} MB`
}

export function KnowledgeBaseModule({ projectId }: ModuleProps) {
  const { t } = useAssistantI18n()
  const [files, setFiles] = useState<KbFileInfo[]>([])
  const [hits, setHits] = useState<KbSearchHitInfo[]>([])
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const fileRef = useRef<HTMLInputElement | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const res = await listKbFiles(projectId)
      setFiles(res.files)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [projectId])

  useEffect(() => {
    void load()
  }, [load])

  const upload = async (file: File | undefined) => {
    if (!file || busy) return
    setBusy(true)
    setError(null)
    setNotice(null)
    try {
      const data = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader()
        reader.onload = () => resolve(String(reader.result))
        reader.onerror = () => reject(reader.error)
        reader.readAsDataURL(file)
      })
      const res = await uploadKbFile(projectId, file.name, data)
      setNotice(
        t('kbUploaded', {
          name: res.name,
          chunks: String(res.ingest.chunks),
        }),
      )
      await load()
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
      if (fileRef.current) fileRef.current.value = ''
    }
  }

  const search = async () => {
    const q = query.trim()
    if (!q || busy) return
    setBusy(true)
    setError(null)
    try {
      const res = await searchKb(projectId, q, 8)
      setHits(res.hits)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={styles.panel}>
      <div className={styles.kbHead}>
        <div>
          <h2>{t('kbTitle')}</h2>
          <p className={styles.muted}>{t('kbHint')}</p>
        </div>
        <input
          ref={fileRef}
          type="file"
          accept={KB_DOC_TYPES.join(',')}
          className={styles.kbFileInput}
          onChange={(e) => void upload(e.target.files?.[0])}
        />
        <button
          type="button"
          className={styles.primary}
          disabled={busy}
          onClick={() => fileRef.current?.click()}
        >
          <Upload size={14} strokeWidth={1.8} aria-hidden />
          {busy ? t('kbUploading') : t('kbUpload')}
        </button>
      </div>

      {error && <div className={styles.error}>{error}</div>}
      {notice && <Notice>{notice}</Notice>}

      <div className={styles.kbSearch}>
        <Search size={15} strokeWidth={1.8} aria-hidden />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t('kbSearchPlaceholder')}
          onKeyDown={(e) => {
            if (e.key === 'Enter') void search()
          }}
        />
        <button
          type="button"
          className={styles.btn}
          disabled={busy || !query.trim()}
          onClick={() => void search()}
        >
          {t('kbSearch')}
        </button>
      </div>

      {hits.length > 0 && (
        <div className={styles.kbSection}>
          <div className={styles.kbSectionHead}>
            <h3>{t('kbHits')}</h3>
            <button
              type="button"
              className={styles.kbClear}
              onClick={() => setHits([])}
            >
              {t('kbClear')}
            </button>
          </div>
          <div className={styles.kbHits}>
            {hits.map((hit, index) => (
              <article key={`${hit.path}-${index}`} className={styles.kbHit}>
                <div className={styles.kbHitHead}>
                  <BookOpen size={13} strokeWidth={1.8} aria-hidden />
                  <strong>
                    <MathMarkdown text={hit.title || hit.path} inline />
                  </strong>
                  <span className={styles.tag}>{hit.path}</span>
                </div>
                {hit.section && (
                  <div className={styles.kbHitSection}>{hit.section}</div>
                )}
                <div className={styles.kbHitText}>
                  <MathMarkdown text={hit.text} />
                </div>
              </article>
            ))}
          </div>
        </div>
      )}

      <div className={styles.kbSection}>
        <div className={styles.kbSectionHead}>
          <h3>{t('kbFiles')}</h3>
          <span className={styles.mono}>{files.length}</span>
        </div>
        {loading ? (
          <Notice>{t('kbLoading')}</Notice>
        ) : files.length === 0 ? (
          <Notice>{t('kbEmpty')}</Notice>
        ) : (
          <div className={styles.kbFiles}>
            {files.map((file) => (
              <div key={file.path} className={styles.kbFile}>
                <div className={styles.kbFileTitle}>
                  <BookOpen size={13} strokeWidth={1.8} aria-hidden />
                  <MathMarkdown text={file.title || file.path} inline />
                </div>
                <span className={styles.kbFilePath}>{file.path}</span>
                <span className={styles.mono}>
                  {formatSize(file.size)}
                  {file.updated_at
                    ? ` · ${new Date(file.updated_at).toLocaleDateString()}`
                    : ''}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  )
}
