import React from "react";
// import Head from "next/head";
// import Link from "next/link";
// import electron from "electron";
import { invoke } from "@tauri-apps/api/tauri";

import styles from "./SourceSelector.module.scss";
import { Box, Button, MenuItem, Select, Typography } from "@mui/material";
// import SourceSelector from "../components/SourceSelector/SourceSelector";
import toBuffer from "blob-to-buffer";
import useAsyncEffect from "use-async-effect";
import { listen } from "@tauri-apps/api/event";

let currentMediaRecorder: MediaRecorder | null = null;

export interface Source {
  hwnd: number;
  title: string;
  rect: {
    top: number;
    left: number;
    right: number;
    bottom: number;
    width: number;
    height: number;
  };
}

export interface SourceData {
  hwnd: string;
  name: string;
  width: number;
  height: number;
  x: number;
  y: number;
}

function SourceSelector({
  projectId = "",
  setProjectId = (valeu: string) => {},
  setCurrentView = (value: string) => {},
}: any) {
  const [sources, setSources] = React.useState<Source[]>([]);
  const [selectedSource, setSelectedSource] = React.useState<number | null>(
    null
  );
  const [isRecording, setIsRecording] = React.useState<boolean>(false);
  const [isLoading, setIsLoading] = React.useState(false);

  const loadSourcePreviews = async () => {
    let sources: Source[] = await invoke("get_sources");

    sources = sources.filter(
      (source) => source.title !== "subshot-rs" && source.title !== ""
    );

    console.info("sources", sources);

    setSources(sources);
  };

  const startRecording = async (sourceHwnd: number) => {
    const source = sources.find((source) => source.hwnd === sourceHwnd);

    if (!source) {
      return;
    }

    const hwnd = source?.hwnd;

    console.info("source", source, hwnd);

    let { projectId }: { projectId: string } = await invoke("create_project");
    let sourceData: SourceData = await invoke("save_source_data", {
      // windowTitle: source.title,
      hwnd,
      currentProjectId: projectId,
    });

    setProjectId(projectId);

    console.info("project", sourceHwnd, projectId, sourceData);

    setIsRecording(true);
    await invoke("start_mouse_tracking");
    await invoke("start_video_capture", {
      hwnd: sourceHwnd,
      width: sourceData.width,
      height: sourceData.height,
      projectId,
    });
  };

  const handleStopRecording = async () => {
    // currentMediaRecorder?.stop();
    setIsRecording(false);
    setIsLoading(true);
    await invoke("stop_mouse_tracking", { projectId });
    await invoke("stop_video_capture", { projectId });

    // wait so video capture can save files and such
  };

  React.useEffect(() => {
    loadSourcePreviews();

    return () => {
      // ipcRenderer.removeAllListeners("ping-pong");
    };
  }, []);

  useAsyncEffect(async () => {
    const unlisten: any = await listen<string>("video-compression", (event) => {
      console.log("video-compression event", event.payload); // Logs: "Hello from the backend!"

      if (event.payload === "success") {
        setIsLoading(false);
        setTimeout(() => {
          setCurrentView("editor");
        }, 500);
      }
    });

    return () => {
      unlisten();
    };
  }, []);

  const handleOpenProject = () => {
    // ipcRenderer.sendSync("open-project");
  };

  const handleStartRecording = () => {
    if (!selectedSource) {
      return;
    }

    startRecording(selectedSource);
  };

  return (
    <>
      <Box className={styles.main}>
        {/* {message} */}
        <Box className={styles.innerContent}>
          <Typography variant="h1">Get Started</Typography>
          <Typography variant="body1">
            SunShot is a screen recording tool that allows you to capture your
            screen and mouse movements in a beautiful way.
          </Typography>

          <Select
            label="Select Source Window"
            placeholder="No source selected"
            style={{
              width: "300px",
            }}
            value={selectedSource ? selectedSource : "init"}
            onChange={(e) => {
              const value = e.target.value as number;

              if (!value) {
                return;
              }

              console.info("update source", value);

              setSelectedSource(value);
            }}
          >
            <MenuItem value={0}>No source selected</MenuItem>
            {sources?.map((source, i) => {
              return <MenuItem value={source.hwnd}>{source.title}</MenuItem>;
            })}
          </Select>

          <Box className={styles.ctrls}>
            {isRecording ? (
              <Button onClick={handleStopRecording}>Stop Recording</Button>
            ) : (
              <Button
                onClick={handleStartRecording}
                disabled={isLoading ? true : selectedSource ? false : true}
              >
                Start Recording
              </Button>
            )}

            {/* <button className={styles.btn} onClick={handleOpenProject}>
              Open a Project
            </button> */}
          </Box>
        </Box>
      </Box>
    </>
  );
}

export default SourceSelector;
