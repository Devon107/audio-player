import { useState } from 'react'
import { coverAssetUrl } from '../../lib/format'
import { MusicNoteIcon } from './Icons'

interface CoverArtProps {
  coverPath: string | null | undefined
  alt: string
  className?: string
  iconClassName?: string
}

export function CoverArt({ coverPath, alt, className, iconClassName }: CoverArtProps) {
  const [failed, setFailed] = useState(false)
  const url = coverAssetUrl(coverPath)

  if (!url || failed) {
    return (
      <div
        className={`flex items-center justify-center bg-app-surface-hover text-app-text-muted ${className ?? ''}`}
      >
        <MusicNoteIcon className={iconClassName ?? 'h-1/2 w-1/2'} />
      </div>
    )
  }

  return (
    <img
      src={url}
      alt={alt}
      className={`object-cover ${className ?? ''}`}
      onError={() => setFailed(true)}
    />
  )
}
