import { Link } from "react-router-dom";
import { Button, MaikoLoader } from "@envenb/ui";
import { useAppContext } from "../lib/context";
import { useI18n } from "../lib/i18n";
import { EmptyState } from "./EmptyState";

/** Empty state pointing at project creation. */
export function NeedsProject() {
  const { t } = useI18n();
  return (
    <EmptyState text={t("guide.needProject")}>
      <Button asChild size="sm">
        <Link to="/projects?new=1">{t("projects.create")}</Link>
      </Button>
    </EmptyState>
  );
}

/** Empty state pointing at the project's environment settings. */
export function NeedsEnvironment({ projectId }: { projectId: string }) {
  const { t } = useI18n();
  return (
    <EmptyState text={t("guide.needEnvironment")}>
      <Button asChild size="sm">
        <Link to={`/projects/${projectId}`}>{t("envs.add")}</Link>
      </Button>
    </EmptyState>
  );
}

/**
 * Renders the guide (or loader) while the global context is not usable yet;
 * otherwise hands the resolved ids to `children`.
 */
export function WithEnvironment({ children }: { children: (ctx: { projectId: string; environmentId: string }) => React.ReactNode }) {
  const { t } = useI18n();
  const { isLoading, projectId, environmentId } = useAppContext();
  if (isLoading) return <MaikoLoader label={t("common.loading")} className="py-16" />;
  if (!projectId) return <NeedsProject />;
  if (!environmentId) return <NeedsEnvironment projectId={projectId} />;
  return <>{children({ projectId, environmentId })}</>;
}
