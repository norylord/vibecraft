const RU: Record<string, string> = Object.fromEntries(
  'а:a,б:b,в:v,г:g,д:d,е:e,ё:e,ж:zh,з:z,и:i,й:y,к:k,л:l,м:m,н:n,о:o,п:p,р:r,с:s,т:t,у:u,ф:f,х:h,ц:c,ч:ch,ш:sh,щ:sch,ъ:,ы:y,ь:,э:e,ю:yu,я:ya'
    .split(',')
    .map(pair => pair.split(':') as [string, string]),
)

/** `DIO-12` + «Статусы агентов» → `DIO-12-statusy-agentov`: git-безопасно, кириллица транслитом */
export function branchName(id: string, summary: string) {
  const slug = summary
    .toLowerCase()
    .replace(/[а-яё]/g, c => RU[c] ?? '')
    .replace(/[^a-z0-9]+/g, '-')
    .slice(0, 40)
    .replace(/^-+|-+$/g, '')
  return slug ? `${id}-${slug}` : id
}
