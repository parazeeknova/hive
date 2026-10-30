export const SceneHexagon = ({ className = "" }: { className?: string }) => (
  <svg
    className={`scene-hexagon ${className}`}
    viewBox="0 0 100 100"
    fill="none"
    aria-hidden="true"
  >
    <path
      d="M46 2.3a8 8 0 0 1 8 0l35.3 20.4a8 8 0 0 1 4 6.9v40.8a8 8 0 0 1-4 6.9L54 97.7a8 8 0 0 1-8 0L10.7 77.3a8 8 0 0 1-4-6.9V29.6a8 8 0 0 1 4-6.9L46 2.3Z"
      fill="currentColor"
    />
    <path
      d="m13 28 34-19a6 6 0 0 1 6 0l34 19"
      stroke="white"
      strokeOpacity=".8"
      strokeWidth="2"
      strokeLinecap="round"
    />
  </svg>
);
