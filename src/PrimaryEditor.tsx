import { Box, Button, CircularProgress } from "@mui/material";
import { invoke } from "@tauri-apps/api/tauri";
import KonvaPreview from "./KonvaPreview";
import { useEffect, useState } from "react";
import {
  Position,
  useEditorContext,
} from "./context/EditorContext/EditorContext";
import { listen } from "@tauri-apps/api/event";
import Tracks from "./Tracks";
import Properties from "./Properties";
import useAsyncEffect from "use-async-effect";

export interface StoredSourceData {
  id: string;
  name: string;
  width: number;
  height: number;
  x: number;
  y: number;
  // scale_factor?
}

export interface ProjectData {
  mousePositions: Position[];
  originalCapture: any;
  sourceData: StoredSourceData;
}

let mounts = 0;

function PrimaryEditor({ projectId = null }: any) {
  const [{ videoTrack, zoomTracks }, dispatch] = useEditorContext();

  const [positions, setPositions] = useState<Position[] | null>(null);
  const [originalCapture, setOriginalCapture] = useState<any>(null);
  const [originalDuration, setOriginalDuration] = useState<number | null>(null);
  const [sourceData, setSourceData] = useState<StoredSourceData | null>(null);

  const getVideoInfo = async () => {
    let { mousePositions, originalCapture, sourceData }: ProjectData =
      await invoke("get_project_data", {
        currentProjectId: projectId,
      });

    const duration = mousePositions[mousePositions.length - 1].timestamp; // may be off by up to 100ms

    console.info(
      "project data",
      projectId,
      mousePositions,
      sourceData,
      // originalCapture,
      duration
    );

    setPositions(mousePositions);
    setOriginalCapture(originalCapture);
    setOriginalDuration(duration);
    setSourceData(sourceData);
  };

  useAsyncEffect(async () => {
    console.info("mounts", mounts);

    if (mounts === 0) {
      getVideoInfo();
    }

    const unlisten: any = await listen<boolean>("video-export", (event) => {
      console.log("video-export event", event.payload); // Logs: "Hello from the backend!"
    });

    mounts++;

    return () => {
      unlisten();
    };
  }, []);

  async function handleTransformVideo() {
    if (!videoTrack?.gradient || !zoomTracks) {
      console.warn("set all settings before export");
      return;
    }

    dispatch({
      key: "exporting",
      value: true,
    });

    await invoke("transform_video", {
      projectId,
      duration: originalDuration,
      zoomInfo: zoomTracks.map((zoomTrack) => {
        return {
          // id: zoomTrack.id,
          // name: zoomTrack.name,
          start: zoomTrack.start,
          end: zoomTrack.end,
          // easing: zoomTrack.easing,
          zoom: zoomTrack.zoomFactor,
        };
      }),
      backgroundInfo: videoTrack.gradient,
    });
  }

  if (!positions || !originalCapture || !sourceData || !originalDuration) {
    return <CircularProgress />;
  }

  return (
    <Box display="flex" flexDirection="column">
      <Box display="flex" flexDirection="row">
        <Box display="flex" flexDirection="column">
          <KonvaPreview
            positions={positions}
            originalCapture={originalCapture}
            sourceData={sourceData}
            resolution={"hd"}
            handleTransformVideo={handleTransformVideo}
          />
          <Tracks originalDuration={originalDuration} />
        </Box>
        <Box>
          <Properties />
        </Box>
      </Box>
    </Box>
  );
}

export default PrimaryEditor;
