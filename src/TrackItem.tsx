import * as React from "react";

// import { TrackItemProps } from "./TrackItem.d";

import Draggable, { DraggableEventHandler } from "react-draggable";
import { Resizable, ResizeCallback } from "re-resizable";
import { useEditorContext } from "./context/EditorContext/EditorContext";

let listenerAttached = 0;

const TrackItem: React.FC<any> = ({
  // constraintsRef = null,
  track = null,
  trackWidth = 1,
  trackHeight = 1,
  originalDuration = 1,
  handleClick = () => console.info("handleClick"),
  updateTrack = () => console.info("updateTrack"),
}) => {
  const [{ zoomTracks }, dispatch] = useEditorContext();

  const { id, start, end } = track;

  const [translating] = React.useState(false);
  const [resizingLeft, setResizingLeft] = React.useState(false);
  const [resizingRight, setResizingRight] = React.useState(false);
  // const [withinLeft, setWithinLeft] = React.useState(false);

  //   const [liveStart, setLiveStart] = React.useState(start);
  //   const [liveEnd, setLiveEnd] = React.useState(end);

  const width = ((end - start) / originalDuration) * 100;
  const widthPx = trackWidth * (width / 100);
  const leftPerc = (start / originalDuration) * 100;
  const left = trackWidth * (leftPerc / 100);
  const msPerPixel = originalDuration / trackWidth;

  // TODO: esc key should cancel any dragging

  React.useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      const { clientX } = e;
      const wrapper = document.getElementById("videoTrackWrapper");
      const targetTrackItem = document.getElementById(id);
      const clickAreaOffset = 200;

      //   console.info("mouse  move", targetTrackItem, translating);

      if (!targetTrackItem) return;

      //   console.info("targetTrackItem", targetTrackItem);

      if (!wrapper) return;

      const wrapperRect = wrapper.getBoundingClientRect();
      const targetTrackItemRect = targetTrackItem.getBoundingClientRect();
      const { left, right, width } = targetTrackItemRect;
      const {
        // left: wrapperLeft,
        // right: wrapperRight,
        width: wrapperWidth,
      } = wrapperRect;

      // const pixelPerMs = originalDuration / wrapperWidth;
      const msPerPixel = wrapperWidth / originalDuration;
      // const baseX = clientX - 200; // -200 to account for offset
      // const totalX = left + (clientX - left);

      //   if (translating) {
      //     const pixelsForX = baseX * pixelPerMs;
      //     const clientXMs = baseX * pixelPerMs;
      //     const widthMs = width * pixelPerMs;
      //     // console.info("msForX", clientX, pixelPerMs, clientXMs, width);
      //     const newStart = Math.floor(clientXMs);
      //     const newEnd = Math.floor(newStart + widthMs); // dont add width to ms times, convert width to ms
      //     updateTrack(
      //       track.id,
      //       "",
      //       [
      //         { key: "start", value: newStart },
      //         { key: "end", value: newEnd },
      //       ],
      //       true
      //     );
      //     // setLiveStart(newStart);
      //     // setLiveEnd(newEnd);
      //   }
      if (resizingLeft) {
        // const newWidth = width - (clientX - left);
        const newLeft = left + (clientX - left);
        // targetTrackItem.style.width = `${newWidth}px`;
        // targetTrackItem.style.left = `${newLeft}px`;
        const newStart = Math.floor(newLeft * msPerPixel);
        // console.info("newStart", newStart, newLeft, msPerPixel);
        updateTrack(track.id, "start", newStart - clickAreaOffset);
      }
      if (resizingRight) {
        const newWidth = width + (clientX - right);
        // targetTrackItem.style.width = `${newWidth}px`;
        const leftSpace = left + newWidth;
        const newEnd = Math.floor(leftSpace * msPerPixel);
        updateTrack(track.id, "end", newEnd + clickAreaOffset);
      }
    };

    listenerAttached++;
    console.info("attach mousemove listener", listenerAttached);
    document.addEventListener("mousemove", handleMouseMove);

    return () => {
      listenerAttached--;
      console.info("detaching mousemove listener");
      document.removeEventListener("mousemove", handleMouseMove);
    };
  }, [translating, resizingLeft, resizingRight]);

  const leftHandleDown = () => {
    console.info("leftHandleDown");
    setResizingLeft(true);
  };

  const leftHandleUp = () => {
    console.info("leftHandleUp");
    setResizingLeft(false);
  };

  // const leftHandleEnter = () => {
  //   console.info("leftHandleEnter");
  //   setWithinLeft(true);
  // };

  // const leftHandleLeave = () => {
  //   console.info("leftHandleLeave");
  //   setWithinLeft(false);
  //   setResizingLeft(false);
  // };

  // const itemHandleDown = (e: React.MouseEvent<HTMLDivElement, MouseEvent>) => {
  //   console.info("itemHandleDown");
  //   setTranslating(true);
  // };

  // const itemHandleUp = () => {
  //   console.info("itemHandleUp");
  //   setTranslating(false);
  // };

  // const itemHandleEnter = () => {
  //   console.info("itemHandleEnter");
  // };

  // const itemHandleLeave = () => {
  //   console.info("itemHandleLeave");
  //   setTranslating(false);
  // };

  const rightHandleDown = () => {
    console.info("rightHandleDown");
    setResizingRight(true);
  };

  const rightHandleUp = () => {
    console.info("rightHandleUp");
    setResizingRight(false);
  };

  const onDrag: DraggableEventHandler = (_, data) => {
    // set live start and end
    const { x } = data;

    console.info("ondragstop", x, msPerPixel);

    const newStart = Math.floor(x * msPerPixel);

    const newEnd = Math.floor((x + widthPx) * msPerPixel);

    updateTrack(
      track.id,
      "",
      [
        { key: "start", value: newStart },
        { key: "end", value: newEnd },
      ],
      true
    );
  };

  const onResize: ResizeCallback = (e: any, side, ref) => {
    const { width } = ref.style;
    const newWidth = parseInt(width);
    const direction = e.movementX > 0 ? "right" : "left";

    console.info("side", side, direction);

    if (side === "left") {
      if (direction === "right") {
        const newStart = Math.floor(
          (left + Math.abs(e.movementX)) * msPerPixel
        );
        updateTrack(track.id, "start", newStart);
      } else {
        const newStart = Math.floor((left + e.movementX) * msPerPixel);
        updateTrack(track.id, "start", newStart);
      }
      // set to match glitch from resizable so bug doesnt bubble up
      // but it should not shorten from right side
      // const newEnd = Math.floor((left + newWidth) * msPerPixel);
      // updateTrack(track.id, "end", newEnd);
    } else if (side === "right") {
      const newEnd = Math.floor((left + newWidth) * msPerPixel);
      updateTrack(track.id, "end", newEnd);
    }
  };

  const handleTrackDelete = () => {
    console.info("handleTrackDelete", track.id);

    const newZoomTracks = zoomTracks?.filter((t) => t.id !== track.id);
    dispatch({ key: "zoomTracks", value: newZoomTracks });
    dispatch({ key: "selectedTrack", value: null });
  };

  return (
    <Draggable
      axis="x"
      handle=".itemHandle"
      // defaultPosition={{ x: left, y: 0 }}
      position={{ x: left, y: 0 }}
      // disabled={true}
      grid={[5, 5]}
      scale={1}
      // onStart={this.handleStart}
      // onDrag={this.handleDrag}
      // onStop={this.handleStop}
      //   drag={resizingLeft || resizingRight ? false : "x"}
      //   dragConstraints={constraintsRef}
      //   onDragEnd={itemDragEnd}
      key={id}
      //   id={id}
      defaultClassName={"item"}
      onMouseDown={() => handleClick(id)}
      onDrag={onDrag}
      // onStop={onDragStop}
    >
      <Resizable
        style={{ position: "absolute" }}
        grid={[1, 1]}
        defaultSize={{
          width: widthPx,
          height: trackHeight,
        }}
        minHeight={trackHeight}
        maxHeight={trackHeight}
        minWidth={5}
        maxWidth={trackWidth}
        onResize={onResize}
        // onResizeStop={onResizeStop}
      >
        <div
          style={
            {
              // left: `${left}%`, // conflicts with drag?
              // width: `${width}%`,
            }
          }
        >
          <div
            className="leftHandle"
            onMouseDown={leftHandleDown}
            onMouseUp={leftHandleUp}
            // onMouseEnter={leftHandleEnter}
            // onMouseLeave={leftHandleLeave}
          ></div>
          <div
            className="itemHandle"
            //   onMouseDown={itemHandleDown}
            //   onMouseUp={itemHandleUp}
            //   onMouseEnter={itemHandleEnter}
            //   onMouseLeave={itemHandleLeave}
          >
            <span className="name">{track.name}</span>
          </div>
          <div className="ctrls">
            <button onClick={handleTrackDelete} title="Remove Zoom Track">
              <i className="ph ph-x"></i>
            </button>
          </div>
          <div
            className="rightHandle"
            onMouseDown={rightHandleDown}
            onMouseUp={rightHandleUp}
          ></div>
        </div>
      </Resizable>
    </Draggable>
  );
};

export default TrackItem;
