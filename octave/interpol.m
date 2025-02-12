clear;

Fs = 48000;
T = 1;
t = linspace(0, T, Fs * T);
F1 = 82;
F2 = 82.3;
F3 = 601;

r = (1:1000);

y1 = sin(2 * pi * F1 * t);
y2 = sin(2 * pi * F2 * t);
y3 = sin(2 * pi * F3 * t);

y1r = y1(r);
y2r = y2(r);
y3r = y3(r);

figure(1); clf; hold on;
plot(y1); plot(y2); plot(y3);

y1f = fft(y1) / Fs;
y2f = fft(y2) / Fs;
y3f = fft(y3) / Fs;

figure(2); clf; hold on;
plot(abs(y1f)); plot(abs(y2f)); plot(abs(y3f));

y1fr = fft(y1r, Fs * 10) / Fs;
y2fr = fft(y2r, Fs * 10) / Fs;
y3fr = fft(y3r, Fs * 10) / Fs;

figure(3); clf; hold on;
%plot(abs(y1fr)); plot(abs(y2fr));
plot(abs(y3fr));

[v1 f1] = max(abs(y1fr))
[v2 f2] = max(abs(y2fr))
[v3 f3] = max(abs(y3fr))
