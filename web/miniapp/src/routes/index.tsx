import { Title } from '@solidjs/meta';
import { createFileRoute } from '@tanstack/solid-router';
import Menu from '../components/Menu.tsrx';

export const Route = createFileRoute('/')({
  component: Home,
});

function Home() {
  return (
    <>
      <Title>Yalom</Title>
      <Menu />
    </>
  );
}