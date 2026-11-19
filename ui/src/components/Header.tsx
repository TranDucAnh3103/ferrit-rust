import React, { useEffect, useRef, useState } from "react";
import {
  GitBranchIcon,
  ShieldCheckIcon,
  ShieldIcon,
  OrganizationIcon,
  ChevronDownIcon,
  CheckIcon,
} from "@primer/octicons-react";
import { Palette, Globe } from "lucide-react";
import { THEME_OPTIONS, type ThemeMode } from "../types/ipc";
import { useI18n } from "../i18n/I18nContext";

interface HeaderProps {
  activeRepo: string;
  activeOwner: string;
  isOrgContext: boolean;
  vaultActive: boolean;
  theme: ThemeMode;
  onThemeChange: (t: ThemeMode) => void;
}

export const Header: React.FC<HeaderProps> = ({
  activeRepo,
  activeOwner,
  isOrgContext,
  vaultActive,
  theme,
  onThemeChange,
}) => {
  const { lang, setLang, t } = useI18n();
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const handleOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handleOutside);
    return () => document.removeEventListener("mousedown", handleOutside);
  }, []);

  const currentTheme = THEME_OPTIONS.find((t) => t.id === theme) || THEME_OPTIONS[0];

  const toggleLanguage = () => {
    setLang(lang === "vi" ? "en" : "vi");
  };

  return (
    <header
      className="bg-canvas-inset border-bottom border-default px-3 d-flex flex-items-center flex-justify-between flex-shrink-0"
      style={{ height: 52 }}
    >
      <div className="d-flex flex-items-center gap-2 mr-3">
        <img
          src="/logo.webp"
          alt="FERRIT"
          style={{ height: 26, width: "auto", objectFit: "contain" }}
          className="flex-shrink-0"
        />
        <span className="text-bold f4 text-primary" style={{ letterSpacing: "0.02em" }}>
          FERRIT
        </span>
        <span className="Label Label--accent tabular-nums">v2.6.0</span>
        {activeOwner && (
          <span className="Label Label--secondary d-flex flex-items-center ml-1">
            {isOrgContext && <OrganizationIcon size={12} className="mr-1" />}
            {activeOwner}
          </span>
        )}
      </div>

      <div
        className="flex-1 d-flex flex-items-center overflow-hidden px-2 py-1 rounded-2 bg-canvas-subtle border border-muted mr-3"
        style={{ maxWidth: 520 }}
      >
        <GitBranchIcon size={14} className="mr-2 text-accent flex-shrink-0" />
        <span className="text-small text-mono text-muted mr-2">{t("header.workspace")}</span>
        <span className="text-small text-mono text-bold text-primary text-truncate">
          {activeRepo || t("header.noRepo")}
        </span>
      </div>

      <div className="d-flex flex-items-center gap-2">
        <span
          className={`d-flex flex-items-center px-2 py-1 rounded-2 text-small border ${
            vaultActive
              ? "text-success border-muted bg-canvas-subtle"
              : "text-muted border-muted bg-canvas-subtle"
          }`}
        >
          {vaultActive ? (
            <ShieldCheckIcon size={14} className="mr-1 text-success" />
          ) : (
            <ShieldIcon size={14} className="mr-1 text-muted" />
          )}
          {vaultActive ? t("header.vaultActive") : t("header.vaultUnconfigured")}
        </span>

        {/* Quick Language Switcher Button */}
        <button
          type="button"
          className="btn btn-sm"
          onClick={toggleLanguage}
          title={t("header.switchLanguage")}
          aria-label={t("header.switchLanguage")}
          style={{ minWidth: 64 }}
        >
          <Globe size={14} className="text-accent flex-shrink-0" />
          <span className="text-bold">{lang === "vi" ? "VI" : "EN"}</span>
        </button>

        {/* Theme Selector Dropdown */}
        <div className="position-relative" ref={menuRef}>
          <button
            type="button"
            className="btn btn-sm"
            aria-haspopup="listbox"
            aria-expanded={menuOpen}
            aria-label={t("header.selectTheme")}
            onClick={() => setMenuOpen((v) => !v)}
          >
            <Palette size={14} />
            <span className="f6">{currentTheme.label}</span>
            <span className="d-flex flex-items-center gap-1 ml-1">
              {currentTheme.swatches.slice(1).map((hex, i) => (
                <span
                  key={i}
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: "50%",
                    backgroundColor: hex,
                    display: "inline-block",
                  }}
                />
              ))}
            </span>
            <ChevronDownIcon size={12} />
          </button>

          {menuOpen && (
            <div className="primer-elevated theme-popover" role="listbox" aria-label="Theme Options">
              <div className="px-2 py-1 text-small text-bold text-muted border-bottom border-muted mb-1">
                {t("settings.themeTitle")}
              </div>
              {THEME_OPTIONS.map((opt) => {
                const selected = opt.id === theme;
                return (
                  <button
                    key={opt.id}
                    type="button"
                    role="option"
                    aria-selected={selected}
                    className={`theme-option-btn ${selected ? "is-selected" : ""}`}
                    onClick={() => {
                      onThemeChange(opt.id);
                      setMenuOpen(false);
                    }}
                  >
                    <div>
                      <div className="f6 text-bold">{opt.label}</div>
                      <div className="text-small text-muted">{opt.description}</div>
                    </div>
                    <div className="d-flex flex-items-center gap-1">
                      {opt.swatches.map((hex, idx) => (
                        <span
                          key={idx}
                          style={{
                            width: 11,
                            height: 11,
                            borderRadius: "50%",
                            backgroundColor: hex,
                            border: "1px solid var(--color-border-default)",
                          }}
                        />
                      ))}
                      {selected && <CheckIcon size={14} className="ml-1 text-accent" />}
                    </div>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </header>
  );
};

export default Header;
