import { useMemo, useState } from "react";
import "./App.css";
import { CircularProgress, createTheme, ThemeProvider } from "@mui/material";
import SourceSelector from "./SourceSelector";
import { getThemeOptions } from "./theme";

function App() {
  const [mode, setMode] = useState<"light" | "dark">("light");
  const theme = useMemo(() => createTheme(getThemeOptions(mode)), [mode]);

  const [currentView, setCurrentView] = useState("source");

  // async function handleTransformVideo() {
  //   // const configPath = "../stubs/tour1/config.json";
  //   const configPath = "../stubs/test1/config.json";
  //   await invoke("transform_video", { configPath })
  // }

  let view = <CircularProgress />;
  switch (currentView) {
    case "source":
      view = <SourceSelector setCurrentView={setCurrentView} />;
      break;

    default:
      view = <CircularProgress />;
      break;
  }

  return <ThemeProvider theme={theme}>{view}</ThemeProvider>;
}

export default App;
