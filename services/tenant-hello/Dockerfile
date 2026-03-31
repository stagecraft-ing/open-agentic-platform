FROM node:20-alpine
WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm ci || npm i
COPY src ./src
EXPOSE 8080
CMD ["npm", "start"]
