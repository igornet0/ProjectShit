import { useEffect, useState } from "react";
import { useGitHubStore } from "@/stores";
import { useTranslation } from "@/i18n";

export function GitHubSettings() {
  const { t } = useTranslation();
  const {
    config,
    loading,
    authMessage,
    oauthUserCode,
    fetchConfig,
    oauthLogin,
    disconnect,
    syncRepos,
    hubEntries,
    fetchHubEntries,
  } = useGitHubStore();
  const [authenticating, setAuthenticating] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  useEffect(() => {
    if (config?.connected) {
      fetchHubEntries();
    }
  }, [config?.connected, fetchHubEntries]);

  const ghostCount = hubEntries.filter((e) => e.kind === "ghost").length;
  const localCount = hubEntries.filter((e) => e.kind === "local").length;

  const handleCopyCode = async () => {
    if (!oauthUserCode) return;
    try {
      await navigator.clipboard.writeText(oauthUserCode);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard may be unavailable
    }
  };

  const handleOAuthLogin = async () => {
    setAuthenticating(true);
    try {
      await oauthLogin();
    } catch {
      // error stored in authMessage
    } finally {
      setAuthenticating(false);
    }
  };

  if (loading && !config) {
    return <p className="muted">{t("common.loading")}</p>;
  }

  return (
    <div className="github-settings">
      {config?.connected ? (
        <div className="github-connected-card">
          <div className="github-connected-header">
            <span className="github-avatar">🐙</span>
            <div>
              <p className="github-connected-title">
                {t("settings.githubConnectedAs", { user: config.username ?? "?" })}
              </p>
              <p className="muted github-connected-meta">
                {t("settings.githubRepoStats", {
                  local: localCount,
                  remote: localCount + ghostCount,
                })}
              </p>
            </div>
          </div>

          {config.default_clone_dir && (
            <p className="muted github-clone-dir">
              {t("settings.githubCloneDir")}: <code>{config.default_clone_dir}</code>
            </p>
          )}

          <div className="github-actions">
            <button
              type="button"
              className="btn"
              disabled={loading}
              onClick={() => syncRepos()}
            >
              {t("settings.githubSync")}
            </button>
            <button
              type="button"
              className="btn danger"
              disabled={loading}
              onClick={() => disconnect()}
            >
              {t("settings.githubDisconnect")}
            </button>
          </div>
        </div>
      ) : (
        <div className="github-connect-card">
          <div className="github-connect-intro">
            <span className="github-logo">🐙</span>
            <div>
              <h3>{t("settings.githubSignInTitle")}</h3>
              <p className="muted">{t("settings.githubHint")}</p>
            </div>
          </div>

          <button
            type="button"
            className="github-sign-in-btn"
            disabled={authenticating || loading}
            onClick={handleOAuthLogin}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true" className="github-mark">
              <path
                fill="currentColor"
                d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.19 0 .21.15.46.55.38A8.013 8.013 0 0 0 16 8c0-4.42-3.58-8-8-8z"
              />
            </svg>
            {authenticating
              ? t("settings.githubOAuthWaiting")
              : t("settings.githubSignIn")}
          </button>

          {authenticating && oauthUserCode && (
            <div className="github-oauth-code-card">
              <p className="github-oauth-code-label">{t("settings.githubOAuthCodeLabel")}</p>
              <div className="github-oauth-code-row">
                <code className="github-oauth-code">{oauthUserCode}</code>
                <button
                  type="button"
                  className="btn github-oauth-copy-btn"
                  onClick={handleCopyCode}
                >
                  {copied ? t("settings.githubOAuthCodeCopied") : t("settings.githubOAuthCodeCopy")}
                </button>
              </div>
              <p className="github-oauth-waiting muted">{t("settings.githubOAuthBrowserHint")}</p>
            </div>
          )}

          {authenticating && !oauthUserCode && (
            <p className="github-oauth-waiting muted">{t("settings.githubOAuthWaiting")}</p>
          )}

          {authMessage && (
            <div className="github-error" role="alert">
              {authMessage}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
