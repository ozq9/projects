/** The Satchel mark: a school bag with a highlighter-yellow buckle. Same drawing as the app icon. */
export function Logo({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 1024 1024" className={className} aria-hidden>
      <path d="M392 338c0-104 240-104 240 0" fill="none" stroke="#e7e5dd" strokeWidth="56" strokeLinecap="round" />
      <rect x="216" y="330" width="592" height="452" rx="96" fill="#e7e5dd" />
      <path d="M216 426a96 96 0 0 1 96-96h400a96 96 0 0 1 96 96v86c-92 58-196 86-296 86s-204-28-296-86z" fill="#b9bfb8" />
      <rect x="466" y="532" width="92" height="112" rx="22" fill="#e8d36b" />
    </svg>
  );
}
