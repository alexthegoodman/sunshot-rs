import * as React from "react";

// import styles from "./Properties.module.scss";
// import shared from "../../pages/shared.module.scss";

// import styled from "styled-components";

// import { PropertiesProps } from "./Properties.d";
import { useEditorContext } from "./context/EditorContext/EditorContext";
import ZoomProperties from "./ZoomProperties";
import VideoProperties from "./VideoProperties";

const Properties: React.FC<any> = () => {
  const [{ videoTrack, zoomTracks, selectedTrack }, dispatch] =
    useEditorContext();

  const trackData =
    videoTrack?.id === selectedTrack
      ? videoTrack
      : (zoomTracks?.find((track) => track.id === selectedTrack) as any);
  const trackKey =
    trackData?.id === videoTrack?.id ? "videoTrack" : "zoomTracks";

  const updateVideoTrack = (key: string, value: any) => {
    dispatch({ key: "videoTrack", value: { ...trackData, [key]: value } });
  };

  const updateZoomTrack = (key: string, value: any) => {
    const updatedZoomTracks = zoomTracks?.map((track) => {
      if (track.id === selectedTrack) {
        return { ...track, [key]: value };
      }
      return track;
    });
    dispatch({ key: "zoomTracks", value: updatedZoomTracks });
  };

  return (
    <section
      // className={`${styles.properties} spectrum-Typography`}
      style={{ padding: "0 25px" }}
    >
      {!trackData ? (
        <div>
          <h1 className="spectrum-Heading spectrum-Heading--sizeL">
            Properties
          </h1>
          <span>Select a track to edit its properties</span>
        </div>
      ) : (
        <>
          {trackKey === "videoTrack" ? (
            <VideoProperties
              trackData={trackData}
              updateTrack={updateVideoTrack}
            />
          ) : (
            <ZoomProperties
              trackData={trackData}
              updateTrack={updateZoomTrack}
            />
          )}
        </>
      )}
    </section>
  );
};

export default Properties;
