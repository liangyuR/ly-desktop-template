import { Link } from "react-router-dom";
import { Alert, Button, Heading, Stack } from "qing-steel";
import type { ReactNode } from "react";
export function PageHeading({
  title,
  children,
}: {
  title: string;
  children?: ReactNode;
}) {
  return (
    <header>
      <Stack justify="space-between">
        <Heading level={1}>{title}</Heading>
        {children && <Stack>{children}</Stack>}
      </Stack>
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
export function ActionLink({ to, children }: { to: string; children: ReactNode }) {
  return (
    <Button asChild variant="text" size="sm">
      <Link to={to}>{children}</Link>
    </Button>
  );
}
export function ProjectLink() {
  return <ActionLink to="/projects">前往项目管理</ActionLink>;
}
