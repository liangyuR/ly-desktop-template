import { Link } from "react-router-dom";
import { Alert, Button } from "qing-steel";
import type { ReactNode } from "react";
export function PageHeading({
  title,
  children,
}: {
  title: string;
  children?: ReactNode;
}) {
  return (
    <header className="page-heading">
      <h1>{title}</h1>
      <div className="actions">{children}</div>
    </header>
  );
}
export function Failure({ text, retry }: { text: string; retry?: () => void }) {
  if (!text) return null;
  return (
    <Alert
      tone="ng"
      action={
        retry && (
          <Button size="sm" onClick={retry}>
            重试
          </Button>
        )
      }
    >
      {text}
    </Alert>
  );
}
export function ProjectLink() {
  return (
    <Link className="inline-link" to="/projects">
      前往项目管理
    </Link>
  );
}
