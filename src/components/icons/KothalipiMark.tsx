// Kothalipi: mono mark in the theme's text colour. Name kept for upstream merges.
const KothalipiMark = ({
  width,
  height,
}: {
  width?: number | string;
  height?: number | string;
}) => (
  <svg
    width={width || 126}
    height={height || 130}
    viewBox="80 80 352 364"
    className="fill-text"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <path
      fillRule="evenodd"
      d="M160 92 H352 Q420 92 420 160 V300 Q420 368 352 368 H222 L132 432 L150 362 Q92 348 92 300 V160 Q92 92 160 92 Z M196 132 H316 V190 Q316 262 256 330 Q196 262 196 190 Z"
    />
    <circle cx="256" cy="222" r="26" />
  </svg>
);

export default KothalipiMark;
