import * as React from "react";

// import styles from "./KonvaPreview.module.scss";

// import { KonvaPreviewProps } from "./KonvaPreview.d";

import { Stage, Layer, Rect, Text, Group, Image, Shape } from "react-konva";
import Konva from "konva";
// import useImage from "use-image";
import { Image as ImageType } from "konva/lib/shapes/Image";
import {
  KonvaEasings,
  Track,
  useEditorContext,
  ZoomTrack,
} from "./context/EditorContext/EditorContext";
import { Box, styled } from "@mui/material";

const ProjectCtrls = styled(Box)`
  display: flex;
  flex-direction: row;
  justify-content: flex-end;
  padding: 15px 0;
`;

const StageContainer = styled(Box)`
  display: flex;
  justify-content: center;
  background-color: lightgray;
`;

const VideoCtrls = styled(Box)`
  display: flex;
  flex-direction: row;
  justify-content: center;
  gap: 15px;
  padding: 15px 0;
`;

let anim: Konva.Animation | null = null;
let zoomInterval: any = null;

// https://stackoverflow.com/questions/59741398/play-video-on-canvas-in-react-konva
const Video = ({
  src,
  positions,
  zoomTracks,
  sourceData,
  playing,
  stopped,
  exporting,
  setCurrentTime,
  divider,
  innerWidth,
  innerHeight,
}: any) => {
  // console.info("videeo", ref);
  const imageRef = React.useRef<ImageType>(null);
  const [size, setSize] = React.useState({ width: 50, height: 50 });
  const [zoomedIn, setZoomedIn] = React.useState(false);

  // we need to use "useMemo" here, so we don't create new video elment on any render
  const videoElement = React.useMemo(() => {
    const element = document.createElement("video");
    const blob = new Blob([src], { type: "video/webm" });
    const url = URL.createObjectURL(blob);
    element.src = url;
    return element;
  }, [src]);

  // when video is loaded, we should read it size
  React.useEffect(() => {
    const onload = function () {
      // setSize({
      //   width: videoElement.videoWidth,
      //   height: videoElement.videoHeight,
      // });
      setSize({
        width: innerWidth,
        height: innerHeight,
      });
    };
    videoElement.addEventListener("loadedmetadata", onload);
    return () => {
      videoElement.removeEventListener("loadedmetadata", onload);
    };
  }, [videoElement]);

  React.useEffect(() => {
    setSize({
      width: innerWidth,
      height: innerHeight,
    });
  }, [innerWidth, innerHeight]);

  const zoomIn = (
    zoomFactor: number,
    zoomPoint: { x: number; y: number },
    easing: KonvaEasings
  ) => {
    // console.info("zoomIn", zoomFactor, zoomPoint);
    setZoomedIn(true);

    imageRef.current?.to({
      scaleX: zoomFactor,
      scaleY: zoomFactor,
      duration: zoomedIn ? 0.1 : 2,
      easing: Konva.Easings[easing],
      // x
      // y
      offsetX: zoomPoint.x,
      offsetY: zoomPoint.y,
    });
  };

  const zoomOut = (easing: KonvaEasings) => {
    // console.info("zoomOut");
    setZoomedIn(false);

    imageRef.current?.to({
      scaleX: 1,
      scaleY: 1,
      duration: 2,
      easing: Konva.Easings[easing],
      // x
      // y
      offsetX: 0,
      offsetY: 0,
    });
  };

  // use Konva.Animation to redraw a layer
  const playCanvasVideo = () => {
    if (zoomTracks && videoElement) {
      // video to canvas animation required
      videoElement.play();
      const layer = imageRef.current?.getLayer();

      anim = new Konva.Animation(() => {}, layer);
      anim.start();

      // mouse follow animation
      // const zoomFactor = 2;
      const refreshRate = 100;
      let point = 0;
      let timeElapsed = 0;

      zoomInterval = setInterval(() => {
        timeElapsed += refreshRate;

        if (!exporting) {
          setCurrentTime(timeElapsed);
        }

        zoomTracks.forEach((track: ZoomTrack) => {
          if (
            Math.floor(timeElapsed) <= Math.floor(track.start) &&
            Math.floor(timeElapsed) + refreshRate > Math.floor(track.start)
          ) {
            const predictionOffset = 0;
            const zoomPoint = {
              x:
                ((positions[point + predictionOffset].x - sourceData.x) /
                  divider) *
                0.8,
              y:
                ((positions[point + predictionOffset].y - sourceData.y) /
                  divider) *
                0.8,
            };

            zoomIn(track.zoomFactor.previewValue, zoomPoint, track.easing);
          }

          if (
            Math.floor(timeElapsed) < Math.floor(track.end) &&
            Math.floor(timeElapsed) + refreshRate >= Math.floor(track.end)
          ) {
            zoomOut(track.easing);
          }
        });

        point++;

        if (point >= positions.length) {
          // zoomOut(videoElement);
          clearInterval(zoomInterval);
        }
      }, refreshRate);

      return () => {
        anim?.stop();
      };
    }
  };

  React.useEffect(() => {
    if (playing) {
      playCanvasVideo();
    } else {
      // stop anim and pause element
      if (anim && videoElement) {
        anim.stop(); // pause()?
        videoElement.pause();
        clearInterval(zoomInterval);
        setCurrentTime(0);
        zoomOut(KonvaEasings.Linear);

        if (stopped) {
          videoElement.currentTime = 0;
        }
      }
    }
  }, [playing, stopped]);

  return (
    <Image
      ref={imageRef}
      image={videoElement}
      x={0}
      y={0}
      // stroke="red"
      width={innerWidth}
      height={innerHeight}
      draggable
      cornerRadius={10}
      // imageSmoothingEnabled={false}
    />
  );
};

const KonvaPreview = ({
  positions = null,
  originalCapture = null,
  sourceData = null,
  resolution = null,
}: any) => {
  const [
    { videoTrack, zoomTracks, currentTime, playing, stopped, exporting },
    dispatch,
  ] = useEditorContext();
  const stageRef = React.useRef(null);
  const layerRef = React.useRef(null);

  const [divider, setDivider] = React.useState(4);

  const width25 = 3840 / divider; // divider of 2 is HD, 1.5 is 2K, 1 is 4K // even 1.5 seems to get cut off
  const height25 = 2160 / divider;
  // const innerWidth = width25 * 0.8;
  // const innerHeight = height25 * 0.8;

  // get resolution, if hd then divvide by 2
  const innerDivider = resolution === "hd" ? divider / 2 : divider;
  const innerWidth = sourceData.width / innerDivider;
  const innerHeight = sourceData.height / innerDivider;

  // console.info("ref", stageRef, layerRef);

  const setCurrentTime = (time: number) => {
    dispatch({ key: "currentTime", value: time });
  };

  const playVideo = () => {
    dispatch({ key: "playing", value: true });
    dispatch({ key: "stopped", value: false });
    // dispatch({ key: "exporting", value: false }); // would be run on recordCanvas
  };

  const stopVideo = () => {
    dispatch({ key: "playing", value: false });
    dispatch({ key: "stopped", value: true });
    dispatch({ key: "exporting", value: false });
  };

  const exportVideo = () => {
    dispatch({ key: "exporting", value: true });
    // setDivider(2);
    // setTimeout(() => {
    //   recordCanvas();
    // }, 1000);
  };
  return (
    <>
      <ProjectCtrls>
        <button
          className="spectrum-Button spectrum-Button--fill spectrum-Button--accent spectrum-Button--sizeM"
          onClick={exportVideo}
        >
          Export
        </button>
      </ProjectCtrls>
      <StageContainer className={`${exporting ? "" : ""}`}>
        <Stage id="stage" ref={stageRef} width={width25} height={height25}>
          <Layer ref={layerRef}>
            <Rect
              width={width25}
              height={height25}
              fillRadialGradientStartPoint={{ x: 0, y: 0 }}
              fillRadialGradientStartRadius={0}
              fillRadialGradientEndPoint={{ x: 0, y: 0 }}
              fillRadialGradientEndRadius={width25}
              fillRadialGradientColorStops={videoTrack?.gradient}
            ></Rect>
            <Rect
              x={width25 / 2 - innerWidth / 2}
              y={height25 / 2 - innerHeight / 2}
              width={innerWidth}
              height={innerHeight}
              fill="black"
              cornerRadius={10}
              shadowColor="black"
              shadowBlur={10}
              shadowOffset={{ x: 10, y: 10 }}
              shadowOpacity={0.5}
            ></Rect>
            <Group
              x={width25 / 2 - innerWidth / 2}
              y={height25 / 2 - innerHeight / 2}
              clipFunc={(ctx) => {
                ctx.rect(0, 0, innerWidth, innerHeight);
              }}
            >
              {/** useEditorContext is not available within <Stage /> */}
              <Video
                src={originalCapture}
                zoomTracks={zoomTracks}
                positions={positions}
                sourceData={sourceData}
                playing={playing}
                stopped={stopped}
                exporting={exporting}
                setCurrentTime={setCurrentTime}
                divider={divider}
                innerWidth={innerWidth}
                innerHeight={innerHeight}
              />
            </Group>
          </Layer>
        </Stage>
      </StageContainer>
      <VideoCtrls>
        <button
          className="spectrum-Button spectrum-Button--fill spectrum-Button--accent spectrum-Button--sizeM"
          onClick={playVideo}
        >
          Play
        </button>
        {/* <button
          onClick={() => {
            dispatch({ key: "playing", value: false });
            dispatch({ key: "stopped", value: false });
          }}
        >
          Pause
        </button> */}
        <button
          className="spectrum-Button spectrum-Button--secondary spectrum-Button--sizeM"
          onClick={stopVideo}
        >
          Stop
        </button>
      </VideoCtrls>
      {/* <video id="recordedCapture" autoPlay={true} loop={true}></video> */}
    </>
  );
};

export default KonvaPreview;