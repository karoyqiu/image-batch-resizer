import '@/App.css';
import { Button } from '@/components/ui/button';

function App() {
  return (
    <main className="flex h-dvh w-dvw flex-col text-center">
      <div className="m-auto flex flex-col gap-4">
        <h1 className="text-primary">Welcome to Tauri + React + TailwindCSS + shadcn/ui!</h1>
        <Button>This is a button</Button>
      </div>
    </main>
  );
}

export default App;
