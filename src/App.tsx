import { useMemo, useState } from "react";
import "./App.css";
import { CircularProgress, createTheme, ThemeProvider } from "@mui/material";
import SourceSelector from "./SourceSelector";
import { getThemeOptions } from "./theme";
import PrimaryEditor from "./PrimaryEditor";

function App() {
  const [mode, setMode] = useState<"light" | "dark">("light");
  const theme = useMemo(() => createTheme(getThemeOptions(mode)), [mode]);

  const [currentView, setCurrentView] = useState("source");
  const [projectId, setProjectId] = useState<string | null>(null);

  let view = <CircularProgress />;
  switch (currentView) {
    case "source":
      view = (
        <SourceSelector
          setCurrentView={setCurrentView}
          projectId={projectId}
          setProjectId={setProjectId}
        />
      );
      break;

    case "editor":
      view = <PrimaryEditor projectId={projectId} />;
      break;

    default:
      view = <CircularProgress />;
      break;
  }

  return <ThemeProvider theme={theme}>{view}</ThemeProvider>;
}

export default App;
