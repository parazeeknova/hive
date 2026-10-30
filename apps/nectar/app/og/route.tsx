import { ImageResponse } from "next/og";

export const dynamic = "force-static";

export const GET = () =>
  new ImageResponse(
    <div
      style={{
        background: "#f6f8ed",
        color: "#20251a",
        display: "flex",
        flexDirection: "column",
        height: "100%",
        justifyContent: "center",
        padding: "80px",
        width: "100%",
      }}
    >
      <div
        style={{
          color: "#526c24",
          display: "flex",
          fontSize: 32,
          fontWeight: 700,
        }}
      >
        HIVE
      </div>
      <div
        style={{
          display: "flex",
          fontSize: 80,
          fontWeight: 700,
          letterSpacing: -4,
          marginTop: 32,
        }}
      >
        Good work, together.
      </div>
      <div
        style={{
          display: "flex",
          fontSize: 30,
          lineHeight: 1.5,
          marginTop: 24,
          maxWidth: 950,
        }}
      >
        Discover useful tools. Connect your agent. Give your best work a second
        life.
      </div>
      <div
        style={{
          color: "#526c24",
          display: "flex",
          fontSize: 24,
          marginTop: 48,
        }}
      >
        hive.itssingularity.com
      </div>
    </div>,
    {
      height: 630,
      width: 1200,
    }
  );
