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
import { Box, Button, styled, Typography } from "@mui/material";

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
// const Video = ({
//   src,
//   positions,
//   zoomTracks,
//   sourceData,
//   playing,
//   stopped,
//   exporting,
//   setCurrentTime,
//   divider,
//   innerWidth,
//   innerHeight,
// }: any) => {
//   // console.info("videeo", ref);
//   const imageRef = React.useRef<ImageType>(null);
//   const [size, setSize] = React.useState({ width: 0, height: 0 });
//   const [zoomedIn, setZoomedIn] = React.useState(false);

//   // we need to use "useMemo" here, so we don't create new video elment on any render
//   const videoElement = React.useMemo(() => {
//     const element = document.createElement("video");
//     // const blob = new Blob([src], { type: "video/mp4" });
//     const blob = new Blob([new Uint8Array(src)], { type: "video/mp4" });
//     const url = URL.createObjectURL(blob);
//     element.src = url;
//     return element;
//   }, [src]);

//   // when video is loaded, we should read it size
//   React.useEffect(() => {
//     const onload = function () {
//       // setSize({
//       //   width: videoElement.videoWidth,
//       //   height: videoElement.videoHeight,
//       // });
//       setSize({
//         width: innerWidth,
//         height: innerHeight,
//       });
//     };
//     videoElement.addEventListener("onload", onload);
//     return () => {
//       videoElement.removeEventListener("onload", onload);
//     };
//   }, [videoElement]);

//   //   React.useEffect(() => {
//   //     setSize({
//   //       width: innerWidth,
//   //       height: innerHeight,
//   //     });
//   //   }, [innerWidth, innerHeight]);

//   const zoomIn = (
//     zoomFactor: number,
//     zoomPoint: { x: number; y: number },
//     easing: KonvaEasings
//   ) => {
//     // console.info("zoomIn", zoomFactor, zoomPoint);
//     setZoomedIn(true);

//     imageRef.current?.to({
//       scaleX: zoomFactor,
//       scaleY: zoomFactor,
//       duration: zoomedIn ? 0.1 : 2,
//       easing: Konva.Easings[easing],
//       // x
//       // y
//       offsetX: zoomPoint.x,
//       offsetY: zoomPoint.y,
//     });
//   };

//   const zoomOut = (easing: KonvaEasings) => {
//     // console.info("zoomOut");
//     setZoomedIn(false);

//     imageRef.current?.to({
//       scaleX: 1,
//       scaleY: 1,
//       duration: 2,
//       easing: Konva.Easings[easing],
//       // x
//       // y
//       offsetX: 0,
//       offsetY: 0,
//     });
//   };

//   // use Konva.Animation to redraw a layer
//   const playCanvasVideo = async () => {
//     console.info("playCanvasVideo", zoomTracks, videoElement);
//     if (zoomTracks && videoElement) {
//       // video to canvas animation required
//       await videoElement.play();
//       const layer = imageRef.current?.getLayer();
//       const imageNode = imageRef.current;

//       anim = new Konva.Animation(() => {
//         // if (videoElement.readyState >= 2) {
//         //   // Ensure video is ready
//         //   const context = layer?.getCanvas().getContext()._context;
//         //   // Draw the current frame of the video onto the canvas
//         //   context?.drawImage(videoElement, 0, 0, size.width, size.height);
//         //   imageNode?.getLayer()?.batchDraw(); // Redraw layer to update the frame
//         // }
//       }, layer);

//       anim.start();

//       // mouse follow animation
//       // const zoomFactor = 2;
//       const refreshRate = 100;
//       let point = 0;
//       let timeElapsed = 0;

//       zoomInterval = setInterval(() => {
//         timeElapsed += refreshRate;

//         if (!exporting) {
//           setCurrentTime(timeElapsed);
//         }

//         zoomTracks.forEach((track: ZoomTrack) => {
//           if (
//             Math.floor(timeElapsed) <= Math.floor(track.start) &&
//             Math.floor(timeElapsed) + refreshRate > Math.floor(track.start)
//           ) {
//             const predictionOffset = 0;
//             const zoomPoint = {
//               x:
//                 ((positions[point + predictionOffset].x - sourceData.x) /
//                   divider) *
//                 0.8,
//               y:
//                 ((positions[point + predictionOffset].y - sourceData.y) /
//                   divider) *
//                 0.8,
//             };

//             zoomIn(track.zoomFactor.previewValue, zoomPoint, track.easing);
//           }

//           if (
//             Math.floor(timeElapsed) < Math.floor(track.end) &&
//             Math.floor(timeElapsed) + refreshRate >= Math.floor(track.end)
//           ) {
//             zoomOut(track.easing);
//           }
//         });

//         point++;

//         if (point >= positions.length) {
//           // zoomOut(videoElement);
//           clearInterval(zoomInterval);
//         }
//       }, refreshRate);

//       return () => {
//         anim?.stop();
//       };
//     }
//   };

//   React.useEffect(() => {
//     if (playing) {
//       playCanvasVideo();
//     } else {
//       // stop anim and pause element
//       if (anim && videoElement) {
//         anim.stop(); // pause()?
//         videoElement.pause();
//         clearInterval(zoomInterval);
//         setCurrentTime(0);
//         zoomOut(KonvaEasings.Linear);

//         if (stopped) {
//           videoElement.currentTime = 0;
//         }
//       }
//     }
//   }, [playing, stopped]);

//   if (!size.width || !size.height) return <></>;

//   return (
//     <Image
//       ref={imageRef}
//       image={videoElement}
//       x={0}
//       y={0}
//       width={size.width}
//       height={size.height}
//       draggable={false}
//       cornerRadius={10}
//       // imageSmoothingEnabled={false}
//     />
//   );
// };

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
  const imageRef = React.useRef<ImageType>(null);
  const [size, setSize] = React.useState({ width: 0, height: 0 });
  const [zoomedIn, setZoomedIn] = React.useState(false);

  // we need to use "useMemo" here, so we don't create new video elment on any render
  const videoElement = React.useMemo(() => {
    const element = document.createElement("video");
    const blob = new Blob([new Uint8Array(src)], { type: "video/mp4" });
    const url = URL.createObjectURL(blob);
    element.src = url;
    // element.src = src;

    element.onerror = (e) => {
      console.error("Error occurred while loading video:", e, element.error);
    };

    return element;
  }, [src]);

  // when video is loaded, we should read it size
  React.useEffect(() => {
    const onload = function () {
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
  //   React.useEffect(() => {

  //   }, [videoElement]);

  const playCanvasVideo = () => {
    console.info("srces", src[0], src[1], src[2]);

    videoElement.play();

    const layer = imageRef?.current?.getLayer();

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
              ((positions[point + predictionOffset].x - sourceData.x) / 2) *
              0.8,
            y:
              ((positions[point + predictionOffset].y - sourceData.y) / 2) * // TODO: scale this. divide by 2 for hd
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
  };

  const stopCanvasVideo = () => {
    // stop anim and pause element
    if (anim && videoElement) {
      anim?.stop(); // pause()?
      videoElement.pause();
      clearInterval(zoomInterval);
      setCurrentTime(0);
      zoomOut(KonvaEasings.Linear);

      if (stopped) {
        videoElement.currentTime = 0;
      }
    }
  };

  React.useEffect(() => {
    if (playing) {
      playCanvasVideo();
    } else {
      stopCanvasVideo();
    }

    return () => {
      stopCanvasVideo();
    };
  }, [playing, stopped]);

  return (
    <Image
      ref={imageRef}
      image={videoElement}
      x={0}
      y={0}
      //   stroke="red"
      width={size.width}
      height={size.height}
      draggable={false}
    />
  );
};

const KonvaPreview = ({
  positions = null,
  originalCapture = null,
  sourceData = null,
  resolution = null,
  handleTransformVideo = () => {},
}: any) => {
  const [
    { videoTrack, zoomTracks, currentTime, playing, stopped, exporting },
    dispatch,
  ] = useEditorContext();
  const stageRef = React.useRef(null);
  const layerRef = React.useRef(null);

  //   const [divider, setDivider] = React.useState(4);

  //   const width25 = 3840 / divider; // divider of 2 is HD, 1.5 is 2K, 1 is 4K // even 1.5 seems to get cut off
  //   const height25 = 2160 / divider;
  // const innerWidth = width25 * 0.8;
  // const innerHeight = height25 * 0.8;

  // get resolution, if hd then divvide by 2
  //   const innerDivider = resolution === "hd" ? divider / 2 : divider;
  const width = 3840 / 4; // divide by 2 for HD, then 2 more for UI inset
  const height = 2160 / 4;
  const proportion = sourceData.width / sourceData.height;
  const innerWidth = (sourceData.width / 2) * 0.8; // already hd, divide by 2 for UI. TODO: don't hardcode
  const innerHeight = (sourceData.height / 2) * 0.8;

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
    handleTransformVideo();
  };

  console.info("gradient", videoTrack?.gradient);

  return (
    <>
      <ProjectCtrls>
        <Button variant="contained" color="success" onClick={exportVideo}>
          Export
        </Button>
      </ProjectCtrls>
      <StageContainer className={`${exporting ? "" : ""}`}>
        <Stage id="stage" ref={stageRef} width={width} height={height}>
          <Layer ref={layerRef}>
            <Rect
              width={width}
              height={height}
              fillRadialGradientStartPoint={{ x: 0, y: 0 }}
              fillRadialGradientStartRadius={0}
              fillRadialGradientEndPoint={{ x: 0, y: 0 }}
              fillRadialGradientEndRadius={width}
              fillRadialGradientColorStops={videoTrack?.gradient?.konvaProps}
            ></Rect>
            {/* <Rect
              x={width / 2 - innerWidth / 2}
              y={height / 2 - innerHeight / 2}
              width={innerWidth}
              height={innerHeight}
              fill="red"
              cornerRadius={10}
              shadowColor="black"
              shadowBlur={10}
              shadowOffset={{ x: 10, y: 10 }}
              shadowOpacity={0.5}
            ></Rect> */}
            <Group
              x={width / 2 - innerWidth / 2}
              y={height / 2 - innerHeight / 2}
              clipFunc={(ctx) => {
                ctx.rect(0, 0, innerWidth, innerHeight);
              }}
            >
              <Video
                src={originalCapture}
                zoomTracks={zoomTracks}
                positions={positions}
                sourceData={sourceData}
                playing={playing}
                stopped={stopped}
                exporting={exporting}
                setCurrentTime={setCurrentTime}
                divider={1}
                innerWidth={innerWidth}
                innerHeight={innerHeight}
              />
            </Group>
          </Layer>
        </Stage>
      </StageContainer>
      <VideoCtrls>
        <Button
          variant="contained"
          color="info"
          size="small"
          onClick={playVideo}
        >
          Play
        </Button>
        {/* <button
          onClick={() => {
            dispatch({ key: "playing", value: false });
            dispatch({ key: "stopped", value: false });
          }}
        >
          Pause
        </button> */}
        <Button
          variant="contained"
          color="info"
          size="small"
          onClick={stopVideo}
        >
          Stop
        </Button>
        <Typography>Current Time: {currentTime}</Typography>
      </VideoCtrls>
      {/* <video id="recordedCapture" autoPlay={true} loop={true}></video> */}
    </>
  );
};

export default KonvaPreview;
