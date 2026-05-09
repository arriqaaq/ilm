import type { NodeHoverDrawingFunction, NodeLabelDrawingFunction } from 'sigma/rendering';

const ARABIC_RE = /[؀-ۿ]/;

function isRTL(text: string): boolean {
  return ARABIC_RE.test(text.charAt(0));
}

function getIsLight(): boolean {
  if (typeof document === 'undefined') return true;
  const theme = document.documentElement.getAttribute('data-theme');
  return !theme || (theme !== 'dark' && theme !== 'sepia');
}

/**
 * Truncate a label until it fits within `maxWidth` pixels, appending an
 * ellipsis. Uses the canvas context's current font for measurement, so call
 * this AFTER setting context.font.
 */
function truncateToWidth(ctx: CanvasRenderingContext2D, label: string, maxWidth: number): string {
  if (maxWidth <= 0) return '';
  if (ctx.measureText(label).width <= maxWidth) return label;
  let lo = 1, hi = label.length, best = '';
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const candidate = label.slice(0, mid).trimEnd() + '…';
    if (ctx.measureText(candidate).width <= maxWidth) {
      best = candidate;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return best || '…';
}

/**
 * Compute the maximum allowed label pixel width for a node. Anchored to the
 * canvas size so labels can never extend past a quarter of the visible canvas
 * width — they get truncated with an ellipsis instead.
 */
function labelBudget(ctx: CanvasRenderingContext2D): number {
  const w = ctx.canvas.width;
  // Conservative cap: a label shouldn't dominate the canvas. Floor at 80px so
  // even on tiny canvases (mobile) at least a few characters render.
  return Math.max(80, Math.floor(w / 4));
}

export const drawLabel: NodeLabelDrawingFunction = (context, data, settings) => {
  if (!data.label) return;

  const size = data.labelSize || settings.labelSize;
  const font = settings.labelFont;
  const weight = settings.labelWeight;
  const color = data.labelColor || settings.labelColor.color;

  context.fillStyle = color;
  context.font = `${weight} ${size}px ${font}`;

  const truncated = truncateToWidth(context, data.label, labelBudget(context));

  if (isRTL(truncated)) {
    context.direction = 'rtl';
    context.textAlign = 'right';
    context.fillText(truncated, data.x - data.size - 3, data.y + size / 3);
  } else {
    context.direction = 'ltr';
    context.textAlign = 'left';
    context.fillText(truncated, data.x + data.size + 3, data.y + size / 3);
  }
};

export const drawHover: NodeHoverDrawingFunction = (context, data, settings) => {
  const size = data.labelSize || settings.labelSize;
  const font = settings.labelFont;
  const weight = settings.labelWeight;
  const light = getIsLight();

  context.font = `${weight} ${size}px ${font}`;
  context.fillStyle = light ? '#FFF' : '#000';

  const PADDING = 4;

  if (typeof data.label === 'string' && data.label) {
    // Hover labels render the FULL untruncated label inside a backing pill,
    // so the user can read long Arabic narrator names on demand.
    const fullLabel = data.label;
    const textWidth = context.measureText(fullLabel).width;
    const boxWidth = Math.round(textWidth + 5);
    const boxHeight = Math.round(size + 2 * PADDING);
    const radius = Math.max(data.size, size / 2) + PADDING;

    const angleRadian = Math.asin(boxHeight / 2 / radius);
    const xDeltaCoord = Math.sqrt(Math.abs(radius ** 2 - (boxHeight / 2) ** 2));

    if (isRTL(fullLabel)) {
      context.beginPath();
      context.moveTo(data.x - xDeltaCoord, data.y + boxHeight / 2);
      context.lineTo(data.x - radius - boxWidth, data.y + boxHeight / 2);
      context.lineTo(data.x - radius - boxWidth, data.y - boxHeight / 2);
      context.lineTo(data.x - xDeltaCoord, data.y - boxHeight / 2);
      context.arc(data.x, data.y, radius, Math.PI - angleRadian, Math.PI + angleRadian, true);
      context.closePath();
      context.fill();
    } else {
      context.beginPath();
      context.moveTo(data.x + xDeltaCoord, data.y + boxHeight / 2);
      context.lineTo(data.x + radius + boxWidth, data.y + boxHeight / 2);
      context.lineTo(data.x + radius + boxWidth, data.y - boxHeight / 2);
      context.lineTo(data.x + xDeltaCoord, data.y - boxHeight / 2);
      context.arc(data.x, data.y, radius, angleRadian, -angleRadian);
      context.closePath();
      context.fill();
    }

    // Render the FULL label inside the pill — bypass drawLabel's truncation.
    const color = data.labelColor || settings.labelColor.color;
    context.fillStyle = color;
    context.font = `${weight} ${size}px ${font}`;
    if (isRTL(fullLabel)) {
      context.direction = 'rtl';
      context.textAlign = 'right';
      context.fillText(fullLabel, data.x - data.size - 3, data.y + size / 3);
    } else {
      context.direction = 'ltr';
      context.textAlign = 'left';
      context.fillText(fullLabel, data.x + data.size + 3, data.y + size / 3);
    }
  } else {
    context.beginPath();
    context.arc(data.x, data.y, data.size + PADDING, 0, Math.PI * 2);
    context.closePath();
    context.fill();
  }
};
