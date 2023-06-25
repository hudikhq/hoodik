export interface Stats {
  mime: string
  size: number
  count: number
}

export interface Response {
  available_space: number
  stats: Stats[]
}
