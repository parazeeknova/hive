import { ImageResponse } from "next/og";

export const dynamic = "force-static";

export const GET = () =>
  new ImageResponse(
    <div tw="flex h-full w-full flex-col justify-center bg-[#f6f8ed] p-[80px] text-[#20251a]">
      <div tw="flex text-[32px] font-bold text-[#526c24]">HIVE</div>
      <div tw="mt-8 flex text-[80px] font-bold tracking-[-4px]">
        Good work, together.
      </div>
      <div tw="mt-6 flex max-w-[950px] text-[30px] leading-[1.5]">
        Discover useful tools. Connect your agent. Give your best work a second
        life.
      </div>
      <div tw="mt-12 flex text-[24px] text-[#526c24]">
        hive.itssingularity.com
      </div>
    </div>,
    { height: 630, width: 1200 }
  );
