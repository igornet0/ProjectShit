import { useEffect, useState } from "react";
import { useGitHubStore } from "@/stores";
import { Icon } from "@/components/Icon/Icon";
import { useTranslation } from "@/i18n";

type GitHubSettingsProps = {
  compact?: boolean;
};

export function GitHubSettings({ compact = false }: GitHubSettingsProps) {
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
    <div className={`github-settings${compact ? " github-settings--compact" : ""}`}>
      {config?.connected ? (
        <>
          <div className="github-account">
            <span className="avatar">
              <Icon name="github" size={18} />
            </span>
            <div>
              <p className="list-row-title">
                {t("settings.githubConnectedAs", { user: config.username ?? "?" })}
              </p>
              <p className="faint">
                {t("settings.githubRepoStats", {
                  local: localCount,
                  remote: localCount + ghostCount,
                })}
              </p>
            </div>
          </div>

          <div className="kv-grid">
            <div className="kv">
              <span className="kv-value">{localCount}</span>
              <span className="kv-label">{t("settings.githubLocal")}</span>
            </div>
            <div className="kv">
              <span className="kv-value">{ghostCount}</span>
              <span className="kv-label">{t("settings.githubRemote")}</span>
            </div>
          </div>

          {config.default_clone_dir && (
            <p className="github-clone-dir">
              {t("settings.githubCloneDir")}: <code>{config.default_clone_dir}</code>
            </p>
          )}

          <div className="button-row">
            <button
              type="button"
              className="btn primary"
              disabled={loading}
              onClick={() => syncRepos()}
            >
              <Icon name="refresh" size={14} />
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
        </>
      ) : (
        <>
          {!compact && (
            <div className="github-account" style={{ marginBottom: 14 }}>
              <span className="avatar">
                <Icon name="github" size={18} />
              </span>
              <div>
                <p className="list-row-title">{t("settings.githubSignInTitle")}</p>
                <p className="faint">{t("settings.githubHint")}</p>
              </div>
            </div>
          )}

          <button
            type="button"
            className="btn primary"
            disabled={authenticating || loading}
            onClick={handleOAuthLogin}
          >
            <Icon name="github" size={14} />
            {authenticating ? t("settings.githubOAuthWaiting") : t("settings.githubSignIn")}
          </button>

          {authenticating && oauthUserCode && (
            <div className="oauth-code">
              <p className="field-label">{t("settings.githubOAuthCodeLabel")}</p>
              <div className="oauth-code-row">
                <code>{oauthUserCode}</code>
                <button type="button" className="btn small" onClick={handleCopyCode}>
                  <Icon name={copied ? "check" : "copy"} size={12} />
                  {copied ? t("settings.githubOAuthCodeCopied") : t("settings.githubOAuthCodeCopy")}
                </button>
              </div>
              <p className="faint">{t("settings.githubOAuthBrowserHint")}</p>
            </div>
          )}

          {authenticating && !oauthUserCode && (
            <p className="faint" style={{ marginTop: 10 }}>
              {t("settings.githubOAuthWaiting")}
            </p>
          )}

          {authMessage && (
            <div className="inline-alert" role="alert">
              {authMessage}
            </div>
          )}
        </>
      )}
    </div>
  );
}
