"use client"

import Image from "next/image";

export default function Home() {
  return (
    <div className='grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-8 pb-20 gap-16 sm:p-20 font-[family-name:var(--font-geist-sans)]'>
      <main className='flex flex-col gap-8 row-start-2 items-center '>
        <Image
          className='dark:invert'
          src='/next.svg'
          alt='Next.js logo'
          width={180}
          height={38}
          priority
        />

        <div className='mb-2 text-2xl'>Plinko Game</div>

        <div className='flex gap-4 items-center flex-col sm:flex-row'>
          <a
            href={"/plinko"}
            className='rounded-full border border-solid border-transparent transition-colors flex items-center justify-center bg-foreground text-background gap-2 hover:bg-[#383838] dark:hover:bg-[#ccc] text-sm sm:text-base h-10 sm:h-12 px-4 sm:px-5'>
            <Image
              className='dark:invert'
              src='/vercel.svg'
              alt='Vercel logomark'
              width={20}
              height={20}
            />
            Play Now
          </a>
        </div>
      </main>
    </div>
  );
}
