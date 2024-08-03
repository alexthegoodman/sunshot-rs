import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

function App() {
  async function handleTransformVideo() {
    const configPath = "../stubs/tour1/config.json";
    await invoke("transform_video", { configPath })
  }

  return (
    <div className="container">
      <h1>Welcome to SunShot (Rust version)!</h1>

      <button onClick={handleTransformVideo}>Transform Video</button>
    </div>
  );
}

export default App;
