"use client";
import { GameProvider } from "@/context/GameContext";
import { PropsWithChildren } from "react";
import { PolkadotProvider } from "@/context/PolkadotContext";

export default function Layout({ children }: PropsWithChildren) {
  return <PolkadotProvider><GameProvider>{children}</GameProvider></PolkadotProvider>;
}
