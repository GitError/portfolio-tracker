import { useEffect, useRef, useState } from 'react';
import { ChevronDown } from 'lucide-react';

export interface SelectOption {
  value: string;
  label: string;
}

interface Props {
  value: string;
  onChange: (value: string) => void;
  options: SelectOption[];
  style?: React.CSSProperties;
}

export function Select({ value, onChange, options, style }: Props) {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  const selectedLabel = options.find((o) => o.value === value)?.label ?? value;

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function onPointerDown(e: PointerEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener('pointerdown', onPointerDown);
    return () => document.removeEventListener('pointerdown', onPointerDown);
  }, [open]);

  // Sync active index to current value when opening
  function handleOpen() {
    const idx = options.findIndex((o) => o.value === value);
    setActiveIndex(idx >= 0 ? idx : 0);
    setOpen(true);
  }

  function handleSelect(optValue: string) {
    onChange(optValue);
    setOpen(false);
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (!open) {
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
        e.preventDefault();
        handleOpen();
      }
      return;
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setActiveIndex((i) => Math.min(i + 1, options.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setActiveIndex((i) => Math.max(i - 1, 0));
        break;
      case 'Enter':
        e.preventDefault();
        handleSelect(options[activeIndex].value);
        break;
      case 'Escape':
      case 'Tab':
        setOpen(false);
        break;
    }
  }

  const triggerStyle: React.CSSProperties = {
    width: '100%',
    background: 'var(--bg-primary)',
    border: '1px solid var(--border-primary)',
    color: 'var(--text-primary)',
    padding: '7px 32px 7px 10px',
    fontSize: 13,
    fontFamily: 'var(--font-mono)',
    borderRadius: '2px',
    cursor: 'pointer',
    textAlign: 'left',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    outline: 'none',
    position: 'relative',
    ...style,
  };

  const dropdownStyle: React.CSSProperties = {
    position: 'absolute',
    top: '100%',
    left: 0,
    right: 0,
    background: 'var(--bg-primary)',
    border: '1px solid var(--border-primary)',
    borderTop: 'none',
    zIndex: 9999,
    maxHeight: 200,
    overflowY: 'auto',
    borderRadius: 0,
  };

  return (
    <div ref={containerRef} style={{ position: 'relative', width: '100%' }}>
      <button
        type="button"
        role="combobox"
        aria-expanded={open}
        aria-haspopup="listbox"
        onClick={() => (open ? setOpen(false) : handleOpen())}
        onKeyDown={handleKeyDown}
        style={triggerStyle}
        onFocus={(e) => (e.currentTarget.style.borderColor = 'var(--color-accent)')}
        onBlur={(e) => {
          e.currentTarget.style.borderColor = 'var(--border-primary)';
        }}
      >
        <span>{selectedLabel}</span>
        <ChevronDown
          size={13}
          style={{
            position: 'absolute',
            right: 10,
            top: '50%',
            transform: `translateY(-50%) rotate(${open ? 180 : 0}deg)`,
            transition: 'transform 0.15s ease',
            color: 'var(--text-secondary)',
            flexShrink: 0,
          }}
        />
      </button>

      {open && (
        <ul role="listbox" style={dropdownStyle}>
          {options.map((opt, idx) => {
            const isSelected = opt.value === value;
            const isActive = idx === activeIndex;
            return (
              <li
                key={opt.value}
                role="option"
                aria-selected={isSelected}
                onPointerDown={(e) => {
                  e.preventDefault();
                  handleSelect(opt.value);
                }}
                onPointerEnter={() => setActiveIndex(idx)}
                style={{
                  padding: '7px 10px',
                  fontSize: 13,
                  fontFamily: 'var(--font-mono)',
                  cursor: 'pointer',
                  color: isSelected ? 'var(--color-accent)' : 'var(--text-primary)',
                  background: isActive ? 'var(--bg-surface-hover)' : 'transparent',
                  userSelect: 'none',
                }}
              >
                {opt.label}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
