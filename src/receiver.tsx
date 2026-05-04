import React from "react";
import ReactDOM from "react-dom/client";
import ReceiverApp from "./ReceiverApp";
import "./App.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ReceiverApp />
  </React.StrictMode>,
);
