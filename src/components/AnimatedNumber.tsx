import { useEffect, useState } from "react";
import CountUp from "react-countup";

interface AnimatedNumberProps {
  value: number;
  duration?: number;
  separator?: string;
  prefix?: string;
  suffix?: string;
  className?: string;
}

export default function AnimatedNumber({
  value,
  duration = 2,
  separator = ",",
  prefix = "",
  suffix = "",
  className = "",
}: AnimatedNumberProps) {
  const [start, setStart] = useState(false);

  useEffect(() => {
    // 延迟启动动画，确保组件已挂载
    const timer = setTimeout(() => setStart(true), 100);
    return () => clearTimeout(timer);
  }, []);

  return (
    <span className={className}>
      {start ? (
        <CountUp
          end={value}
          duration={duration}
          separator={separator}
          prefix={prefix}
          suffix={suffix}
        />
      ) : (
        "0"
      )}
    </span>
  );
}
