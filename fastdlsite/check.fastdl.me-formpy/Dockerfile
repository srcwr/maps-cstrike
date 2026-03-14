FROM alpine:latest
WORKDIR /app

RUN apk add wget zip
RUN wget https://cosmo.zip/pub/cosmos/bin/assimilate-x86_64.elf https://redbean.dev/redbean-3.0.0.com
RUN chmod +x *.elf *.com
RUN ./assimilate-x86_64.elf -c redbean-3.0.0.com

COPY init.lua ./.init.lua
RUN zip redbean-3.0.0.com .init.lua

CMD ["./redbean-3.0.0.com", "-X", "-p", "55154"]
