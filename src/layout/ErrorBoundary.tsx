import { Component, type ReactNode } from "react";
import { Button, Card, EmptyState } from "qing-steel";
export class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state: { error: Error | null } = { error: null };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  render() {
    return this.state.error ? (
      <Card role="alert">
        <EmptyState
          title="页面出错了"
          description={this.state.error.message}
          action={
            <Button onClick={() => this.setState({ error: null })}>重试</Button>
          }
        />
      </Card>
    ) : (
      this.props.children
    );
  }
}
