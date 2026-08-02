import type { SVGProps } from 'react'

// Set mínimo de íconos como SVG inline (sin dependencia externa de íconos). Todos aceptan las
// props estándar de SVG para poder pasar className/size desde donde se usan.

type IconProps = SVGProps<SVGSVGElement>

const base = {
  viewBox: '0 0 24 24',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 2,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
}

export function PlayIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <polygon points="6 3 20 12 6 21 6 3" fill="currentColor" stroke="none" />
    </svg>
  )
}

export function PauseIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <rect x="6" y="4" width="4" height="16" fill="currentColor" stroke="none" />
      <rect x="14" y="4" width="4" height="16" fill="currentColor" stroke="none" />
    </svg>
  )
}

export function NextIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <polygon points="5 4 15 12 5 20 5 4" fill="currentColor" stroke="none" />
      <rect x="17" y="4" width="2.5" height="16" fill="currentColor" stroke="none" />
    </svg>
  )
}

export function PreviousIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <polygon points="19 4 9 12 19 20 19 4" fill="currentColor" stroke="none" />
      <rect x="4.5" y="4" width="2.5" height="16" fill="currentColor" stroke="none" />
    </svg>
  )
}

export function ShuffleIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M17 3h4v4" />
      <path d="M21 3l-7 7" />
      <path d="M3 17l4-4" />
      <path d="M3 3l6.5 6.5" />
      <path d="M21 21l-6.5-6.5" />
      <path d="M17 21h4v-4" />
    </svg>
  )
}

export function RepeatIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M17 2l4 4-4 4" />
      <path d="M3 11V9a4 4 0 0 1 4-4h14" />
      <path d="M7 22l-4-4 4-4" />
      <path d="M21 13v2a4 4 0 0 1-4 4H3" />
    </svg>
  )
}

export function RepeatOneIcon(props: IconProps) {
  // El dígito solo (fontSize 8, sin fondo) se confundía con una "T" a tamaños pequeños. Un
  // círculo relleno de currentColor detrás, con el número en el color de la superficie de la
  // app (contraste fijo), hace que el modo "repetir pista" se distinga de "repetir cola" de un
  // vistazo, sin depender del tooltip.
  return (
    <svg {...base} {...props}>
      <path d="M17 2l4 4-4 4" />
      <path d="M3 11V9a4 4 0 0 1 4-4h14" />
      <path d="M7 22l-4-4 4-4" />
      <path d="M21 13v2a4 4 0 0 1-4 4H3" />
      <circle cx="12" cy="12" r="5.5" fill="currentColor" stroke="none" />
      <text
        x="12"
        y="15.5"
        fontSize="9"
        fontWeight="700"
        fill="#17181d"
        stroke="none"
        textAnchor="middle"
      >
        1
      </text>
    </svg>
  )
}

export function VolumeIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <polygon points="4 9 8 9 12 5 12 19 8 15 4 15 4 9" fill="currentColor" stroke="none" />
      <path d="M16 8a5 5 0 0 1 0 8" />
      <path d="M18.5 5.5a9 9 0 0 1 0 13" />
    </svg>
  )
}

export function VolumeMuteIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <polygon points="4 9 8 9 12 5 12 19 8 15 4 15 4 9" fill="currentColor" stroke="none" />
      <path d="M16 9l5 6" />
      <path d="M21 9l-5 6" />
    </svg>
  )
}

export function SearchIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <circle cx="11" cy="11" r="7" />
      <path d="M21 21l-4.3-4.3" />
    </svg>
  )
}

export function PlusIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M12 5v14" />
      <path d="M5 12h14" />
    </svg>
  )
}

export function TrashIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M3 6h18" />
      <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
      <path d="M10 11v6" />
      <path d="M14 11v6" />
    </svg>
  )
}

export function CloseIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M18 6L6 18" />
      <path d="M6 6l12 12" />
    </svg>
  )
}

export function FolderIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
    </svg>
  )
}

export function MusicNoteIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M9 18V5l12-2v13" />
      <circle cx="6" cy="18" r="3" />
      <circle cx="18" cy="16" r="3" />
    </svg>
  )
}

export function ChevronDownIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M6 9l6 6 6-6" />
    </svg>
  )
}

export function GripIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <circle cx="9" cy="6" r="1.3" fill="currentColor" stroke="none" />
      <circle cx="15" cy="6" r="1.3" fill="currentColor" stroke="none" />
      <circle cx="9" cy="12" r="1.3" fill="currentColor" stroke="none" />
      <circle cx="15" cy="12" r="1.3" fill="currentColor" stroke="none" />
      <circle cx="9" cy="18" r="1.3" fill="currentColor" stroke="none" />
      <circle cx="15" cy="18" r="1.3" fill="currentColor" stroke="none" />
    </svg>
  )
}

export function DownloadIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M12 3v12" />
      <path d="M7 10l5 5 5-5" />
      <path d="M4 21h16" />
    </svg>
  )
}

export function UploadIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M12 21V9" />
      <path d="M7 14l5-5 5 5" />
      <path d="M4 21h16" />
    </svg>
  )
}

export function ListIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M8 6h13" />
      <path d="M8 12h13" />
      <path d="M8 18h13" />
      <path d="M3 6h.01" />
      <path d="M3 12h.01" />
      <path d="M3 18h.01" />
    </svg>
  )
}

export function MoreIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <circle cx="12" cy="5" r="1.3" fill="currentColor" stroke="none" />
      <circle cx="12" cy="12" r="1.3" fill="currentColor" stroke="none" />
      <circle cx="12" cy="19" r="1.3" fill="currentColor" stroke="none" />
    </svg>
  )
}
