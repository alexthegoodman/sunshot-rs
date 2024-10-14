import React from "react";
// import Head from "next/head";
// import Link from "next/link";
// import electron from "electron";
import { invoke } from "@tauri-apps/api/tauri";

import styles from "./SourceSelector.module.scss";
import { Box, Button, MenuItem, Select, Typography } from "@mui/material";
// import SourceSelector from "../components/SourceSelector/SourceSelector";
import toBuffer from "blob-to-buffer";

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

    // const hwnd = source.id.split(":")[1];
    const hwnd = source?.hwnd;

    console.info("source", source, hwnd);

    // const { projectId } = ipcRenderer.sendSync("create-project");
    // const sourceData = ipcRenderer.sendSync("save-source-data", {
    //   windowTitle: source.name,
    //   hwnd,
    // });

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

    /**navigator.mediaDevices
      .getUserMedia({
        audio: false,
        video: {
          mandatory: {
            chromeMediaSource: "desktop",
            chromeMediaSourceId: sourceHwnd,
            width: sourceData.width,
            height: sourceData.height,
            minFrameRate: 60,
            maxFrameRate: 60,
          },
        },
      } as any)
      .then(async (stream) => {
        console.info("stream", stream);

        const streamSettings = stream.getVideoTracks()[0].getSettings();
        const streamWidth = stream.getVideoTracks()[0].getSettings().width;
        const streamHeight = stream.getVideoTracks()[0].getSettings().height;

        console.info(
          "stream settings",
          streamSettings,
          JSON.stringify(streamSettings),
          streamWidth,
          streamHeight
        );

        const stopRecording = async () => {
          setIsRecording(false);
          // clearInterval(captureInterval);
          // ipcRenderer.sendSync("stop-mouse-tracking", { projectId });
          await invoke("stop_mouse_tracking", { projectId });
          console.info("stop-mouse-tracking");

          stream.getTracks()[0].stop();
          const blob = new Blob(chunks, { type: "video/webm" });

          // const arrayBuffer = await blob.arrayBuffer();
          // const buffer = Buffer.from(arrayBuffer);
          // const newBlob = new Blob([buffer]);

          // console.info("blog chunks", blob, chunks, buffer);

          // ipcRenderer.sendSync("save-video-blob", {
          //   projectId,
          //   buffer,
          //   sourceId,
          // });

          toBuffer(blob, async function (err, buffer) {
            if (err) throw err;

            await invoke("save_video_blob", { projectId, buffer });

            console.info("save-video-blob");

            setCurrentView("editor");
          });

          // ipcRenderer.sendSync("close-source-picker");
          // ipcRenderer.sendSync("open-editor", { projectId });
        };

        const chunks: any = [];
        currentMediaRecorder = new MediaRecorder(stream, {
          mimeType: "video/webm; codecs=vp9",
        });
        currentMediaRecorder.ondataavailable = (e) => chunks.push(e.data);
        currentMediaRecorder.onerror = (e) =>
          console.error("mediaRecorder error", e);
        currentMediaRecorder.onstop = stopRecording;
        currentMediaRecorder.start();

        // ipcRenderer.sendSync("start-mouse-tracking");
        await invoke("start_mouse_tracking");

        setIsRecording(true);

        console.info("start-mouse-tracking");
      })
      .catch((error) => console.log(error));*/
  };

  const handleStopRecording = async () => {
    // currentMediaRecorder?.stop();
    setIsRecording(false);
    await invoke("stop_mouse_tracking", { projectId });
    await invoke("stop_video_capture");
    setCurrentView("editor");
  };

  React.useEffect(() => {
    loadSourcePreviews();

    return () => {
      // ipcRenderer.removeAllListeners("ping-pong");
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
                disabled={selectedSource ? false : true}
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
