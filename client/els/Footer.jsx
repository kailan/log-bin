export default function Footer(props) {
  return (
    <footer>
      <code>curl -s -XPOST '{location.origin}/{props.bucketID}' -H 'Content-type: text/plain' -d 'A test log message!'</code><br/>
      <code><strong>msg=</strong>foo|bar</code> Fields to extract as main log message &nbsp;
      <code><strong>meta=</strong>foo|bar</code> Only show these fields &nbsp;
      <code><strong>time=</strong>foo|bar</code> Fields to extract as time
    </footer>
  );
}
