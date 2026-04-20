import "./App.css";
import Button from "./components/Button";

function App() {
  return (
    <main className="flex flex-col bg-[#1F1F1E] items-center">
      <img src="/sharing.svg" />
      <span className="h1 text-white">SharingYourSounds</span>
      <form>
        <input type="placeholder" />
        <Button>Connect</Button>
      </form>
    </main>
  );
}

export default App;
